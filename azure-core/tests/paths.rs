// Le dossier des sockets : cree prive, et refuse s'il est modifiable par
// d'autres comptes (un autre compte pourrait y poser un faux daemon).
use azure_core::paths::prepare_socket_dir;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[test]
fn the_socket_dir_is_private_or_refused() {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("paths");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();

    let dir = base.join("run");
    prepare_socket_dir(dir.join("manager.sock").to_str().unwrap()).unwrap();
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);

    let open = base.join("ouvert");
    std::fs::create_dir_all(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o777)).unwrap();
    let err = prepare_socket_dir(open.join("manager.sock").to_str().unwrap()).unwrap_err();
    assert!(err.contains("modifiable par d'autres"), "{err}");
}
