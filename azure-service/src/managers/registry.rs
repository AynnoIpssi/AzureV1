// Quelle app est quel executable : le premier executable qui annonce un id
// le garde (comme azure-stockage). Une autre app ne peut donc pas publier
// dans le flux d'une autre, ni ecouter a sa place. Fichier texte
// `apps.txt` : une ligne `id<TAB>executable` par app.
use std::path::PathBuf;

pub struct AppRegistry {
    file: PathBuf,
}

impl AppRegistry {
    pub fn open(dir: &std::path::Path) -> Result<AppRegistry, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        Ok(AppRegistry { file: dir.join("apps.txt") })
    }

    fn read(&self) -> Result<Vec<(u32, String)>, String> {
        let text = match std::fs::read_to_string(&self.file) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(format!("{} : {e}", self.file.display())),
        };
        Ok(text.lines().filter_map(|line| line.split_once('\t')).filter_map(|(id, exe)| Some((id.parse().ok()?, exe.to_string()))).collect())
    }

    /// Lie `app` a `exe` la premiere fois ; ensuite, refuse tout autre
    /// executable.
    pub fn bind(&self, app: u32, exe: &str) -> Result<(), String> {
        let mut apps = self.read()?;
        match apps.iter().find(|(id, _)| *id == app) {
            Some((_, known)) if known == exe => Ok(()),
            Some((_, known)) => Err(format!("L'app {app} appartient a l'executable {known}, pas a {exe}")),
            None => {
                apps.push((app, exe.to_string()));
                let text: String = apps.iter().map(|(id, exe)| format!("{id}\t{exe}\n")).collect();
                let tmp = self.file.with_extension("tmp");
                std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &self.file)).map_err(|e| format!("{} : {e}", self.file.display()))
            }
        }
    }
}
