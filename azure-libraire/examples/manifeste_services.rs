// Ecrit les sections `[provide ...]` de tous les services de la librairie :
// `cargo run -p azure-libraire --example manifeste_services [tache] [acces]`.
fn main() {
    let mut args = std::env::args().skip(1);
    let tache = args.next().unwrap_or_else(|| "methodes".to_string());
    let acces = args.next().unwrap_or_else(|| "public = true".to_string());
    print!("{}", azure_libraire::service::manifeste(azure_libraire::service::services(), &tache, &acces));
}
