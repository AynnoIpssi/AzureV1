// Octets aleatoires du noyau (`getrandom`, sans fichier a ouvrir : marche
// aussi dans un bac a sable sans /dev), pour les cles, nonces et sels.

// Declaree ici plutot que par la crate `libc` : la librairie ne depend de
// rien, et la bibliotheque C est deja liee par `std`.
unsafe extern "C" {
    fn getrandom(buf: *mut core::ffi::c_void, buflen: usize, flags: core::ffi::c_uint) -> isize;
}

pub fn random_bytes(out: &mut [u8]) -> Result<(), String> {
    let mut filled = 0;
    while filled < out.len() {
        // SAFETY : on ecrit au plus `out.len() - filled` octets dans `out`.
        let n = unsafe { getrandom(out[filled..].as_mut_ptr().cast(), out.len() - filled, 0) };
        if n < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(format!("getrandom : {e}"));
        }
        filled += n as usize;
    }
    Ok(())
}
