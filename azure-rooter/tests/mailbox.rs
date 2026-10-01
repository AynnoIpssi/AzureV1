// Messages envoyes a une app qui ne tourne pas : gardes puis livres a son
// enregistrement, meme apres un redemarrage du routeur.
use azure_rooter::managers::router::start_router_with;
use azure_rooter::models::mailbox::{Mailbox, MAX_PER_APP};
use azure_rooter::services::client::{receive, register_at, send};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn start(name: &str, data: &std::path::Path) -> String {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-rooter-mailbox-{name}-{}.sock"), std::process::id(), name = name);
    let (s, d) = (socket.clone(), data.to_path_buf());
    std::thread::spawn(move || start_router_with(&s, Some(&d)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    socket
}

fn data(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("mailbox-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn messages_wait_for_the_app_and_survive_a_restart() {
    let dir = data("restart");
    let first = start("a", &dir);
    let mut sender = register_at(&first, 2).unwrap();
    send(&mut sender, 2, 7, "bonjour").unwrap();
    send(&mut sender, 2, 7, "/commande/42\u{1f}payload").unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Un autre routeur (redemarrage) sur le meme dossier.
    let second = start("b", &dir);
    let mut app = register_at(&second, 7).unwrap();
    assert_eq!(receive(&mut app).unwrap(), "bonjour");
    assert_eq!(receive(&mut app).unwrap(), "/commande/42\u{1f}payload", "dans l'ordre d'envoi");
    // Livres une seule fois.
    assert_eq!(Mailbox::open(&dir).unwrap().count(7), 0);
}

#[test]
fn an_app_that_is_running_gets_messages_directly() {
    let dir = data("direct");
    let socket = start("c", &dir);
    let mut app = register_at(&socket, 9).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let mut sender = register_at(&socket, 1).unwrap();
    send(&mut sender, 1, 9, "tout de suite").unwrap();
    assert_eq!(receive(&mut app).unwrap(), "tout de suite");
    assert_eq!(Mailbox::open(&dir).unwrap().count(9), 0);
}

#[test]
fn the_mailbox_keeps_the_newest_messages() {
    let mut mailbox = Mailbox::in_memory();
    for i in 0..(MAX_PER_APP + 10) {
        mailbox.push(3, format!("m{i}"));
    }
    let messages = mailbox.take(3);
    assert_eq!(messages.len(), MAX_PER_APP);
    assert_eq!(messages[0], "m10");
    assert!(mailbox.take(3).is_empty());
}
