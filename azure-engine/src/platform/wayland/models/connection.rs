use std::io::Write;
use std::os::fd::RawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::io::AsRawFd;



pub struct WaylandConnection {
    stream: UnixStream,
    // Descripteurs recus avec les messages (`SCM_RIGHTS`), dans l'ordre
    // d'arrivee. Le compositeur peut les joindre a un envoi qui contient
    // plusieurs messages : ils ne suivent donc pas forcement l'octet du
    // message qui les porte, et chaque evenement a argument `fd` prend le
    // suivant de la file (comme libwayland). Voir `take_fd`.
    fds: std::collections::VecDeque<RawFd>,
    // `wl_keyboard` : `read_message` lit au passage son `keymap` (la carte
    // XKB de la disposition de l'utilisateur, voir `take_keymap`) et ses
    // `modifiers` (voir `keyboard_modifiers`).
    keyboard_id: Option<u32>,
    // Derniere carte recue, pas encore lue par l'app.
    keymap: Option<String>,
    // `(verrouilles, groupe)` du dernier `wl_keyboard::modifiers`.
    modifiers: (u32, u32),
    // Messages deja lus mais mis de cote pendant une attente synchrone
    // (voir `defer`) : `read_message` les rend d'abord, dans l'ordre.
    pending: std::collections::VecDeque<(u32, u16, Vec<u8>)>,
}

impl WaylandConnection {

    pub fn stream(&self) -> &UnixStream { &self.stream }
    pub fn new(stream: UnixStream) -> WaylandConnection {
        WaylandConnection { stream, fds: std::collections::VecDeque::new(), keyboard_id: None, keymap: None, modifiers: (0, 0), pending: std::collections::VecDeque::new() }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), String> {
        self.stream.write_all(data)
            .map_err(|e| e.to_string())
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), String> {
        self.stream.set_nonblocking(nonblocking)
            .map_err(|e| e.to_string())
    }

    pub fn send_with_fd(&mut self, data: &[u8], fd: RawFd) -> Result<(), String> {
        let iov = libc::iovec {
            iov_base: data.as_ptr() as *mut libc::c_void,
            iov_len: data.len(),
        };



        let cmsg_buffer_len = unsafe {
            libc::CMSG_SPACE(std::mem::size_of::<RawFd>() as u32) as usize
        };
        let mut cmsg_buffer = vec![0u8; cmsg_buffer_len];

        let msghdr = libc::msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &iov as *const libc::iovec as *mut libc::iovec,
            msg_iovlen: 1,
            msg_control:  cmsg_buffer.as_mut_ptr() as *mut libc::c_void,
            msg_flags: 0,
            msg_controllen: cmsg_buffer_len,
        };

        let cmsg_ptr = unsafe {
            libc::CMSG_FIRSTHDR(&msghdr)
        };

        if cmsg_ptr.is_null() {
            return Err("Failed to get cmsg header".to_string());
        }

        unsafe {
            (*cmsg_ptr).cmsg_level = libc::SOL_SOCKET;
            (*cmsg_ptr).cmsg_type = libc::SCM_RIGHTS;
        }

        unsafe {
            (*cmsg_ptr).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<RawFd>() as u32) as usize;
        }

        unsafe {
            let data_ptr = libc::CMSG_DATA(cmsg_ptr) as *mut RawFd;
            std::ptr::write(data_ptr, fd);
        }

        let socket_fd = self.stream.as_raw_fd();

        let result = unsafe {
            libc::sendmsg(socket_fd, &msghdr, 0)
        };

        if result == -1 {
            return Err("Failed to send message with fd".to_string());
        }

        Ok(())

    }

    /// Lit exactement `buf.len()` octets, en gardant les descripteurs qui
    /// arrivent avec (voir `take_fd`) - un simple `read` les fermerait.
    pub fn receive(&mut self, buf: &mut [u8]) -> Result<(), String> {
        let mut filled = 0;
        while filled < buf.len() {
            let mut iov = libc::iovec { iov_base: buf[filled..].as_mut_ptr() as *mut libc::c_void, iov_len: buf.len() - filled };
            // Place pour quelques fd a la fois (un message en porte au plus un).
            let mut control = [0u8; 128];
            let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
            msg.msg_iov = &mut iov;
            msg.msg_iovlen = 1;
            msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
            msg.msg_controllen = control.len();
            let n = unsafe { libc::recvmsg(self.stream.as_raw_fd(), &mut msg, libc::MSG_CMSG_CLOEXEC) };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    // Socket non bloquant (voir `set_nonblocking`) : le reste
                    // du message arrive, on l'attend.
                    self.wait_readable(-1)?;
                    continue;
                }
                return Err(err.to_string());
            }
            if n == 0 {
                return Err("connexion Wayland fermee".to_string());
            }
            unsafe {
                let mut cmsg = libc::CMSG_FIRSTHDR(&msg);
                while !cmsg.is_null() {
                    if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                        let data = libc::CMSG_DATA(cmsg) as *const RawFd;
                        let count = ((*cmsg).cmsg_len as usize - libc::CMSG_LEN(0) as usize) / std::mem::size_of::<RawFd>();
                        for i in 0..count {
                            self.fds.push_back(std::ptr::read_unaligned(data.add(i)));
                        }
                    }
                    cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
                }
            }
            filled += n as usize;
        }
        Ok(())
    }

    /// Le prochain descripteur recu (voir `fds`), a fermer par qui le prend.
    pub fn take_fd(&mut self) -> Option<RawFd> {
        self.fds.pop_front()
    }

    /// Voir `keyboard_id`.
    pub fn set_keyboard_id(&mut self, id: u32) {
        self.keyboard_id = Some(id);
    }

    /// La carte XKB (texte `xkb_keymap { ... }`) recue depuis le dernier
    /// appel : le compositeur l'envoie a l'ouverture, et de nouveau quand
    /// l'utilisateur change de disposition.
    pub fn take_keymap(&mut self) -> Option<String> {
        self.keymap.take()
    }

    /// `(modificateurs verrouilles, groupe)` du clavier : le bit 2 des
    /// verrouilles est Verr. Maj dans les cartes XKB usuelles.
    pub fn keyboard_modifiers(&self) -> (u32, u32) {
        self.modifiers
    }

    /// Lit un message complet : `(objet, opcode, arguments)`. Lit au
    /// passage la carte et les modificateurs du clavier (voir `keyboard_id`).
    pub fn read_message(&mut self) -> Result<(u32, u16, Vec<u8>), String> {
        if let Some(message) = self.pending.pop_front() {
            return Ok(message);
        }
        let mut header = [0u8; 8];
        self.receive(&mut header)?;
        let object_id = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let size_opcode = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let size = (size_opcode >> 16) as usize;
        let opcode = (size_opcode & 0xFFFF) as u16;
        if size < 8 {
            return Err(format!("message Wayland trop court ({size} octets)"));
        }
        let mut args = vec![0u8; size - 8];
        self.receive(&mut args)?;
        if Some(object_id) == self.keyboard_id && opcode == 0
            && let Some(fd) = self.take_fd()
        {
            // format (1 = xkb_v1), taille ; le fd arrive a part.
            let format = u32::from_le_bytes(args[0..4].try_into().unwrap_or([0; 4]));
            let size = u32::from_le_bytes(args.get(4..8).and_then(|b| b.try_into().ok()).unwrap_or([0; 4])) as usize;
            if format == 1 && let Some(text) = read_keymap(fd, size) {
                self.keymap = Some(text);
            }
            unsafe { libc::close(fd) };
        }
        if Some(object_id) == self.keyboard_id && opcode == 4 && args.len() >= 20 {
            // serial, enfonces, verrouilles temporairement, verrouilles, groupe
            let at = |i: usize| u32::from_le_bytes(args[i..i + 4].try_into().unwrap());
            self.modifiers = (at(12), at(16));
        }
        Ok((object_id, opcode, args))
    }

    // Regarde si au moins un octet est disponible, sans le consommer (MSG_PEEK).
    // Permet de savoir si un message est en train d'arriver avant de faire une
    // lecture bloquante complete, sans jamais risquer de perdre des octets
    // sur une lecture partielle non bloquante.
    /// Remet un message lu pendant une attente synchrone (ex. un jeton
    /// d'activation, voir `Window::activation_token`) : la boucle
    /// d'evenements le traitera normalement ensuite.
    pub fn defer(&mut self, message: (u32, u16, Vec<u8>)) {
        self.pending.push_back(message);
    }

    pub fn has_data(&mut self) -> Result<bool, String> {
        if !self.pending.is_empty() {
            return Ok(true);
        }
        let socket_fd = self.stream.as_raw_fd();
        let mut probe = [0u8; 1];
        let result = unsafe {
            libc::recv(
                socket_fd,
                probe.as_mut_ptr() as *mut libc::c_void,
                probe.len(),
                libc::MSG_PEEK | libc::MSG_DONTWAIT,
            )
        };
        if result >= 0 {
            Ok(result > 0)
        } else {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::WouldBlock {
                Ok(false)
            } else {
                Err(err.to_string())
            }
        }
    }

    // Attend jusqu'a `timeout_ms` qu'un message soit disponible, sans le
    // consommer - a la difference de `has_data` (qui repond tout de suite,
    // present ou pas), celle-ci dort reellement jusqu'a l'un des deux :
    // donnee disponible, ou timeout ecoule. C'est ce qui permet a la
    // boucle d'evenements de se reveiller toute seule (clignotement du
    // curseur, repetition de touche) meme quand rien n'arrive du
    // compositeur - `receive` seul (lecture bloquante sans limite de
    // temps) ne le permettait pas.
    pub fn wait_readable(&self, timeout_ms: i32) -> Result<bool, String> {
        if !self.pending.is_empty() {
            return Ok(true);
        }
        let mut pfd = libc::pollfd {
            fd: self.stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            // Un signal (ex. `kill`, voir azure_core::security::termination) :
            // rien a lire, la boucle passe a son tic.
            if error.kind() == std::io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error.to_string());
        }
        Ok(result > 0 && (pfd.revents & libc::POLLIN) != 0)
    }
}

// Le texte de la carte : le fd est une memoire partagee, lue en lecture
// seule (`MAP_PRIVATE`, comme l'exige le protocole depuis la version 7).
fn read_keymap(fd: RawFd, size: usize) -> Option<String> {
    if size == 0 || size > 16 * 1024 * 1024 {
        return None;
    }
    let ptr = unsafe { libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ, libc::MAP_PRIVATE, fd, 0) };
    if ptr == libc::MAP_FAILED {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(size);
    let text = String::from_utf8_lossy(&bytes[..end]).into_owned();
    unsafe { libc::munmap(ptr, size) };
    Some(text)
}
