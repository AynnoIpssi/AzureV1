// azure-provider cote app : les daemons d'Azure (routeur, stockage) sont
// lances a la demande, et une app peut confier ses propres taches de fond.
//
// ```text
// use azure_foundation::provider::{Provider, Service};
//
// Service::new("notes-sync", "notes_sync")      // relance si elle tombe
//     .arg("--rapide")
//     .health_socket("/tmp/notes-sync.sock")
//     .register()?;
// Provider::ensure("notes-sync")?;              // attend qu'elle reponde
// for s in Provider::status()? { println!("{} : {}", s.name, s.state.label()); }
// ```
pub use azure_provider::{Provider, Restart, Service, ServiceStatus, State};

/// Demande au provider de s'assurer que `service` tourne, puis `connect`.
/// Si le provider n'y arrive pas (binaire absent...) on tente quand meme :
/// un daemon lance a la main suffit. L'erreur du provider est ajoutee a
/// celle de la connexion pour savoir pourquoi.
pub(crate) fn with_service<T>(service: &str, connect: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let ensured = Provider::ensure(service);
    connect().map_err(|e| match ensured {
        Err(provider) => format!("{e} (azure-provider : {provider})"),
        Ok(()) => e,
    })
}
