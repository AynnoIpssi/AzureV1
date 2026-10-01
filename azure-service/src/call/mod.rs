pub mod broker;
pub mod client;

pub use client::{CallRequest, Server, DEFAULT_TIMEOUT};

/// Delai maximal d'un appel.
pub const MAX_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
