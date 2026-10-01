// Le coeur du provider : l'etat de chaque service, et `tick` qui, appele
// regulierement, detecte les arrets, sonde les sockets et relance.
//
// Toutes les decisions prennent l'heure en parametre (`now`) : les tests
// avancent le temps sans attendre.
use crate::managers::process::{describe_exit, health_ok, spawn, terminate};
use crate::models::policy::{CrashHistory, RestartPolicy};
use crate::models::service::{is_builtin, Restart, ServiceSpec, ServiceStatus, State};
use std::path::PathBuf;
use std::process::Child;
use std::time::Instant;

struct Supervised {
    spec: ServiceSpec,
    child: Option<Child>,
    state: State,
    /// Lancement du processus en cours.
    launched: Option<Instant>,
    /// Depuis quand il est pret.
    ready_since: Option<Instant>,
    last_check: Option<Instant>,
    failed_checks: u32,
    crashes: CrashHistory,
    restarts: u32,
    retry_at: Option<Instant>,
    message: String,
}

impl Supervised {
    fn new(spec: ServiceSpec) -> Supervised {
        Supervised {
            spec,
            child: None,
            state: State::Stopped,
            launched: None,
            ready_since: None,
            last_check: None,
            failed_checks: 0,
            crashes: CrashHistory::default(),
            restarts: 0,
            retry_at: None,
            message: String::new(),
        }
    }
}

pub struct Supervisor {
    services: Vec<Supervised>,
    policy: RestartPolicy,
    log_dir: PathBuf,
}

impl Supervisor {
    pub fn new(specs: Vec<ServiceSpec>, policy: RestartPolicy, log_dir: PathBuf) -> Supervisor {
        Supervisor { services: specs.into_iter().map(Supervised::new).collect(), policy, log_dir }
    }

    fn find(&mut self, name: &str) -> Result<&mut Supervised, String> {
        self.services.iter_mut().find(|s| s.spec.name == name).ok_or_else(|| format!("Service inconnu : '{name}'"))
    }

    /// Lance tous les services `autostart`.
    pub fn start_autostart(&mut self, now: Instant) {
        for index in 0..self.services.len() {
            if self.services[index].spec.autostart {
                self.launch(index, now);
            }
        }
    }

    /// Lance le service s'il ne tourne pas. Remet a zero un service en
    /// echec (c'est la relance a la main).
    pub fn start(&mut self, name: &str, now: Instant) -> Result<(), String> {
        let index = self.index(name)?;
        let service = &mut self.services[index];
        if matches!(service.state, State::Starting | State::Running | State::External) {
            return Ok(());
        }
        service.crashes.clear();
        self.launch(index, now);
        Ok(())
    }

    /// Arrete le service ; il n'est plus relance jusqu'au prochain `start`.
    /// Un service externe (pas lance par le provider) n'est pas tue : il
    /// n'est simplement plus surveille.
    pub fn stop(&mut self, name: &str) -> Result<(), String> {
        let timeout = self.policy.stop_timeout;
        let service = self.find(name)?;
        if let Some(mut child) = service.child.take() {
            terminate(&mut child, timeout);
        }
        service.state = State::Stopped;
        service.ready_since = None;
        service.retry_at = None;
        service.message = "arrete a la demande".to_string();
        Ok(())
    }

    pub fn restart(&mut self, name: &str, now: Instant) -> Result<(), String> {
        self.stop(name)?;
        self.start(name, now)
    }

    /// Ajoute un service (demande d'une app). Deja connu avec la meme
    /// definition : simplement lance s'il ne tourne pas. Definition
    /// differente : remplace et relance. Un daemon d'Azure ne peut pas etre
    /// remplace ainsi (seulement par la config).
    pub fn register(&mut self, spec: ServiceSpec, now: Instant) -> Result<(), String> {
        spec.validate()?;
        match self.index(&spec.name) {
            Ok(index) if self.services[index].spec == spec => self.start(&spec.name, now),
            Ok(_) if is_builtin(&spec.name) => Err(format!("'{}' est un daemon d'Azure : il se modifie dans provider.conf", spec.name)),
            Ok(index) => {
                self.stop(&spec.name)?;
                self.services[index] = Supervised::new(spec.clone());
                self.start(&spec.name, now)
            }
            Err(_) => {
                self.services.push(Supervised::new(spec.clone()));
                self.start(&spec.name, now)
            }
        }
    }

    /// Fait connaitre un service sans le lancer : il le sera a la
    /// demande. Une definition differente remplace l'ancienne, sans arreter
    /// le processus en cours : elle vaut pour son prochain lancement.
    pub fn declare(&mut self, spec: ServiceSpec) -> Result<(), String> {
        spec.validate()?;
        match self.index(&spec.name) {
            Ok(index) if self.services[index].spec == spec => Ok(()),
            Ok(_) if is_builtin(&spec.name) => Err(format!("'{}' est un daemon d'Azure : il se modifie dans provider.conf", spec.name)),
            Ok(index) => {
                self.services[index].spec = spec;
                Ok(())
            }
            Err(_) => {
                self.services.push(Supervised::new(spec));
                Ok(())
            }
        }
    }

    pub fn unregister(&mut self, name: &str) -> Result<(), String> {
        if is_builtin(name) {
            return Err(format!("'{name}' est un daemon d'Azure : il se desactive dans provider.conf"));
        }
        self.stop(name)?;
        self.services.retain(|s| s.spec.name != name);
        Ok(())
    }

    pub fn state(&self, name: &str) -> Option<State> {
        self.services.iter().find(|s| s.spec.name == name).map(|s| s.state)
    }

    /// Message du dernier arret ou echec.
    pub fn message(&self, name: &str) -> Option<String> {
        self.services.iter().find(|s| s.spec.name == name).map(|s| s.message.clone())
    }

    pub fn status(&self, now: Instant) -> Vec<ServiceStatus> {
        self.services
            .iter()
            .map(|s| ServiceStatus {
                name: s.spec.name.clone(),
                state: s.state,
                pid: s.child.as_ref().map(|c| c.id()),
                restarts: s.restarts,
                uptime_secs: s.ready_since.map(|t| now.saturating_duration_since(t).as_secs()).unwrap_or(0),
                message: s.message.clone(),
            })
            .collect()
    }

    /// Detecte les arrets, sonde les sockets, relance ce qui doit l'etre.
    pub fn tick(&mut self, now: Instant) {
        for index in 0..self.services.len() {
            self.tick_one(index, now);
        }
    }

    /// Arrete tous les services lances par le provider (fin du provider).
    pub fn shutdown(&mut self) {
        let names: Vec<String> = self.services.iter().map(|s| s.spec.name.clone()).collect();
        for name in names {
            let _ = self.stop(&name);
        }
    }

    fn index(&self, name: &str) -> Result<usize, String> {
        self.services.iter().position(|s| s.spec.name == name).ok_or_else(|| format!("Service inconnu : '{name}'"))
    }

    fn launch(&mut self, index: usize, now: Instant) {
        let service = &mut self.services[index];
        service.retry_at = None;
        service.failed_checks = 0;
        service.last_check = Some(now);
        // Deja lance hors du provider (a la main, ou laisse par un provider
        // precedent) : on le surveille au lieu d'en lancer un second.
        if let Some(socket) = &service.spec.health
            && health_ok(socket) {
                service.state = State::External;
                service.ready_since = Some(now);
                return;
            }
        match spawn(&service.spec, &self.log_dir) {
            Ok(child) => {
                service.child = Some(child);
                service.launched = Some(now);
                if service.spec.health.is_some() {
                    service.state = State::Starting;
                    service.ready_since = None;
                } else {
                    service.state = State::Running;
                    service.ready_since = Some(now);
                }
            }
            Err(message) => self.crashed(index, message, now),
        }
    }

    /// Le service vient de tomber : relance plus tard, ou abandon.
    fn crashed(&mut self, index: usize, reason: String, now: Instant) {
        let policy = self.policy.clone();
        let service = &mut self.services[index];
        if let Some(mut child) = service.child.take() {
            terminate(&mut child, policy.stop_timeout);
        }
        service.ready_since = None;
        let recent = service.crashes.record(now, policy.window);
        if policy.gives_up(recent) {
            service.state = State::Failed;
            service.message = format!("{reason} ; {recent} plantages en {} s, abandon", policy.window.as_secs());
            return;
        }
        service.message = reason;
        service.restarts += 1;
        let delay = policy.delay(recent);
        if delay.is_zero() {
            self.launch(index, now);
        } else {
            let service = &mut self.services[index];
            service.state = State::Backoff;
            service.retry_at = Some(now + delay);
        }
    }

    fn tick_one(&mut self, index: usize, now: Instant) {
        let policy = self.policy.clone();
        let service = &mut self.services[index];

        // Sorti de lui-meme ?
        if let Some(child) = service.child.as_mut()
            && let Ok(Some(status)) = child.try_wait() {
                service.child = None;
                let reason = describe_exit(status);
                let relaunch = match service.spec.restart {
                    Restart::Always => true,
                    Restart::OnFailure => !status.success(),
                    Restart::Never => false,
                };
                if relaunch {
                    return self.crashed(index, reason, now);
                }
                service.state = State::Exited;
                service.ready_since = None;
                service.message = reason;
                return;
            }

        match service.state {
            State::Backoff if service.retry_at.is_some_and(|t| now >= t) => self.launch(index, now),
            State::Starting => {
                let socket = service.spec.health.clone().unwrap_or_default();
                if health_ok(&socket) {
                    service.state = State::Running;
                    service.ready_since = Some(now);
                    service.last_check = Some(now);
                } else if service.launched.is_some_and(|t| now.saturating_duration_since(t) >= policy.start_timeout) {
                    let reason = format!("ne repond pas sur {socket} apres {} s", policy.start_timeout.as_secs());
                    self.crashed(index, reason, now);
                }
            }
            State::Running | State::External => {
                let Some(socket) = service.spec.health.clone() else { return };
                if service.last_check.is_some_and(|t| now.saturating_duration_since(t) < policy.health_interval) {
                    return;
                }
                service.last_check = Some(now);
                if health_ok(&socket) {
                    service.failed_checks = 0;
                    return;
                }
                service.failed_checks += 1;
                if service.failed_checks < policy.health_failures {
                    return;
                }
                if service.state == State::External {
                    // Pas a nous, pas un plantage a compter : on prend le relais.
                    service.message = format!("le daemon externe ne repond plus sur {socket}, relance par le provider");
                    self.launch(index, now);
                } else {
                    let reason = format!("ne repond plus sur {socket}");
                    self.crashed(index, reason, now);
                }
            }
            _ => {}
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}
