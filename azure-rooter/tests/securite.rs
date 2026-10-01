// Le routeur ne croit pas un client sur parole : l'id annonce doit etre le
// sien (azure-manager, ou premier executable arrive), une connexion n'agit
// qu'en son nom, et une taille annoncee demesuree coupe la connexion.
use azure_core::models::wire::{Reader, Writer};
use azure_rooter::managers::router::{set_manager_socket, start_router_with};
use azure_rooter::services::client::{receive, register_at, send, subscribe, publish, unregister};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Un faux azure-manager : l'app 1500 est un AUTRE executable.
fn fake_manager() -> String {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-rooter-secu-manager-{}.sock"), std::process::id());
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).unwrap();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut size = [0u8; 4];
            if stream.read_exact(&mut size).is_err() {
                continue;
            }
            let mut payload = vec![0u8; u32::from_le_bytes(size) as usize];
            let _ = stream.read_exact(&mut payload);
            let mut r = Reader::new(&payload);
            let (_op, id) = (r.u32().unwrap(), r.u32().unwrap());
            let response = if id == 1500 {
                Writer::new().u8(0).u8(1).str("autre").str("/usr/bin/autre").u8(0).bytes(&[]).u8(1).finish()
            } else {
                Writer::new().u8(0).u8(0).finish()
            };
            let _ = stream.write_all(&(response.len() as u32).to_le_bytes()).and_then(|_| stream.write_all(&response));
        }
    });
    socket
}

fn start() -> String {
    set_manager_socket(&fake_manager());
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-rooter-secu-{}.sock"), std::process::id());
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rooter-secu");
    let _ = std::fs::remove_dir_all(&data);
    let s = socket.clone();
    std::thread::spawn(move || start_router_with(&s, Some(&data)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    socket
}

fn closed(stream: &mut UnixStream) -> bool {
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut b = [0u8; 1];
    match stream.read(&mut b) {
        Ok(0) => true,
        Err(e) => !matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut),
        Ok(_) => false,
    }
}

#[test]
fn le_routeur_ne_croit_pas_les_clients_sur_parole() {
    let socket = start();

    // 1500 est connue d'azure-manager comme un autre executable : refusee.
    let mut imposteur = register_at(&socket, 1500).unwrap();
    assert!(closed(&mut imposteur), "l'imposteur doit etre coupe");

    // Une connexion n'agit qu'en son nom : A (20) ne desinscrit pas B (21),
    // ni ne l'abonne a sa place.
    let mut b = register_at(&socket, 21).unwrap();
    let mut a = register_at(&socket, 20).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    unregister(&mut a, 21).unwrap();
    subscribe(&mut a, 21, "piege").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    send(&mut a, 20, 21, "toujours la").unwrap();
    assert_eq!(receive(&mut b).unwrap(), "toujours la", "B n'a pas ete desinscrite par A");
    publish(&mut a, 20, "piege", "ne doit pas arriver").unwrap();
    send(&mut a, 20, 21, "suivant").unwrap();
    assert_eq!(receive(&mut b).unwrap(), "suivant", "B n'a pas ete abonnee par A");

    // Taille annoncee de 4 Go : connexion coupee, rien d'alloue.
    let mut vorace = register_at(&socket, 22).unwrap();
    let mut frame = Vec::new();
    frame.extend_from_slice(&1u32.to_le_bytes());
    frame.extend_from_slice(&22u32.to_le_bytes());
    frame.extend_from_slice(&21u32.to_le_bytes());
    frame.extend_from_slice(&u32::MAX.to_le_bytes());
    vorace.write_all(&frame).unwrap();
    assert!(closed(&mut vorace), "message demesure : connexion coupee");

    // Le routeur continue de servir les autres.
    send(&mut a, 20, 21, "encore").unwrap();
    assert_eq!(receive(&mut b).unwrap(), "encore");
}
