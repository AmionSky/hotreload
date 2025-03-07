use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub trait Reload {
    type Data: serde::de::DeserializeOwned;
    fn apply(&self, data: Self::Data) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Hotreload<C> {
    config: Arc<C>,
    _watcher: RecommendedWatcher,
}

impl<C> Hotreload<C>
where
    C: Reload + Default + Send + Sync + 'static,
{
    pub fn new<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        // Get path of the config file containing directory
        let path: PathBuf = path.into();
        let watch_path = path.parent().ok_or(Error::NoParent)?.to_path_buf();

        // Init config type
        let config = Arc::new(C::default());
        let config_clone = config.clone();

        // Load config file
        Self::reload(&config, &path)?;

        // Create & Start file watcher
        type NotifyRes = notify::Result<notify::Event>;
        let mut watcher = notify::recommended_watcher(move |res: NotifyRes| match res {
            Ok(event) => {
                if event.paths.len() == 1
                    && event.paths[0] == path
                    && (event.kind.is_modify() || event.kind.is_create())
                {
                    if let Err(error) = Self::reload(&config_clone, &path) {
                        eprintln!("Failed to hotreload config: {}", error);
                    }
                }
            }
            Err(error) => eprintln!("Hotreload watch error: {}", error),
        })?;
        watcher.watch(&watch_path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            config,
            _watcher: watcher,
        })
    }

    pub fn config(&self) -> &Arc<C> {
        &self.config
    }

    fn reload<P: AsRef<Path>>(config: &C, path: P) -> Result<(), Error> {
        let file = load_file(path)?;
        let data = toml::from_str(&file).map_err(Error::Deserialize)?;
        config.apply(data).map_err(Error::Apply)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Config file not found: {0}")]
    NotFound(#[source] std::io::Error),
    #[error("Config file permission denied: {0}")]
    PermissionDenied(#[source] std::io::Error),
    #[error("Failed to read config file: {0}")]
    FileRead(#[source] std::io::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to deserialize config TOML: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("Path doesn't have a parent")]
    NoParent,
    #[error("Failed to apply new config: {0}")]
    Apply(#[source] Box<dyn std::error::Error>),
}

fn load_file<P: AsRef<Path>>(path: P) -> Result<String, Error> {
    // Open file
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Err(match error.kind() {
                ErrorKind::NotFound => Error::NotFound(error),
                ErrorKind::PermissionDenied => Error::PermissionDenied(error),
                _ => Error::Io(error),
            });
        }
    };

    // Read content
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).map_err(Error::FileRead)?;
    Ok(buffer)
}
