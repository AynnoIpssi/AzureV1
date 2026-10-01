// Les donnees MINIMALES de toute fenetre Azure, quelle que soit l'app qui la
// cree : qui la cree (`owner_app_id`), sa taille, son etat, qui a le droit
// de la voir (`WindowScope`) et ce qu'elle est (`WindowKind`). Le
// constructeur `WindowSpec::new` refuse toute combinaison incoherente :
// une fenetre ne peut pas exister avec des donnees contradictoires.
//
// Exemple : l'app A cree une fenetre pour elle-meme ->
// `WindowSpec::new(A, size, Active, WindowScope::Owner, WindowKind::Internal)`.
// L'app A envoie une fenetre a l'app B -> `WindowKind::External` avec
// `WindowScope::Followers` ou `WindowScope::All`.

/// Taille d'une fenetre en pixels. Jamais nulle (voir `WindowSize::new`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    width: u32,
    height: u32,
}

impl WindowSize {
    pub fn new(width: u32, height: u32) -> Result<WindowSize, String> {
        if width == 0 || height == 0 {
            return Err(format!("Window size cannot be zero ({width}x{height})"));
        }
        Ok(WindowSize { width, height })
    }

    pub fn width(&self) -> u32 { self.width }

    pub fn height(&self) -> u32 { self.height }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Active,
    Background,
}

/// Qui a le droit de voir / recevoir la fenetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowScope {
    /// Uniquement l'app qui l'a creee.
    Owner,
    /// L'app creatrice et les apps abonnees a elle (si A suit B, A voit
    /// les fenetres `Followers` de B - voir le `follow` d'azure-rooter).
    Followers,
    /// Toutes les apps.
    All,
}

impl WindowScope {
    /// Code sur le fil (routeur azure-rooter, fenetres envoyees entre apps) -
    /// une seule definition partagee par l'emetteur et le routeur.
    pub fn code(&self) -> u32 {
        match self {
            WindowScope::Owner => 0,
            WindowScope::Followers => 1,
            WindowScope::All => 2,
        }
    }

    pub fn from_code(code: u32) -> Option<WindowScope> {
        match code {
            0 => Some(WindowScope::Owner),
            1 => Some(WindowScope::Followers),
            2 => Some(WindowScope::All),
            _ => None,
        }
    }
}

impl WindowState {
    pub fn code(&self) -> u32 {
        match self {
            WindowState::Active => 0,
            WindowState::Background => 1,
        }
    }

    pub fn from_code(code: u32) -> Option<WindowState> {
        match code {
            0 => Some(WindowState::Active),
            1 => Some(WindowState::Background),
            _ => None,
        }
    }
}

/// Ce qu'est la fenetre par rapport a l'app qui la cree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    /// Creee par une app pour elle-meme.
    Internal,
    /// Creee par une app pour etre envoyee a une AUTRE app.
    External,
    /// Partagee entre plusieurs apps, l'app creatrice comprise.
    InterApp,
}

impl WindowKind {
    pub fn code(&self) -> u32 {
        match self {
            WindowKind::Internal => 0,
            WindowKind::External => 1,
            WindowKind::InterApp => 2,
        }
    }

    pub fn from_code(code: u32) -> Option<WindowKind> {
        match code {
            0 => Some(WindowKind::Internal),
            1 => Some(WindowKind::External),
            2 => Some(WindowKind::InterApp),
            _ => None,
        }
    }

    /// Les scopes autorises pour ce type de fenetre : une fenetre interne
    /// reste dans son app, une fenetre externe ou inter-app doit pouvoir
    /// sortir de l'app creatrice.
    pub fn allows(&self, scope: WindowScope) -> bool {
        match self {
            WindowKind::Internal => scope == WindowScope::Owner,
            WindowKind::External | WindowKind::InterApp => scope != WindowScope::Owner,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSpec {
    owner_app_id: u32,
    size: WindowSize,
    state: WindowState,
    scope: WindowScope,
    kind: WindowKind,
}

impl WindowSpec {
    pub fn new(owner_app_id: u32, size: WindowSize, state: WindowState, scope: WindowScope, kind: WindowKind) -> Result<WindowSpec, String> {
        if !kind.allows(scope) {
            return Err(format!("A {kind:?} window cannot have the {scope:?} scope"));
        }
        Ok(WindowSpec { owner_app_id, size, state, scope, kind })
    }

    /// Raccourci pour le cas le plus courant : une fenetre interne, active,
    /// visible uniquement par l'app qui la cree.
    pub fn internal(owner_app_id: u32, size: WindowSize) -> WindowSpec {
        WindowSpec { owner_app_id, size, state: WindowState::Active, scope: WindowScope::Owner, kind: WindowKind::Internal }
    }

    pub fn owner_app_id(&self) -> u32 { self.owner_app_id }

    pub fn size(&self) -> WindowSize { self.size }

    pub fn state(&self) -> WindowState { self.state }

    pub fn scope(&self) -> WindowScope { self.scope }

    pub fn kind(&self) -> WindowKind { self.kind }

    /// Seul champ modifiable apres creation : l'etat change pendant la vie
    /// de la fenetre, le reste la definit.
    pub fn set_state(&mut self, state: WindowState) {
        self.state = state;
    }

    /// `true` si l'app `app_id` a le droit de voir cette fenetre.
    /// `follows_owner` : `app_id` suit-elle l'app creatrice (voir
    /// azure-rooter) ? C'est a l'appelant de le savoir, le core ne connait
    /// pas le routeur.
    pub fn visible_to(&self, app_id: u32, follows_owner: bool) -> bool {
        if app_id == self.owner_app_id {
            return true;
        }
        match self.scope {
            WindowScope::Owner => false,
            WindowScope::Followers => follows_owner,
            WindowScope::All => true,
        }
    }
}
