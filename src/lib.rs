use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

pub trait HotreloadApply<D> {
    fn apply(&self, data: D);
}

pub struct Hotreload<C, D> {
    config: Arc<C>,
    _watcher: RecommendedWatcher,
    _data: PhantomData<D>,
}

impl<C, D> Hotreload<C, D>
where
    C: HotreloadApply<D> + Default + Send + Sync + 'static,
    D: serde::de::DeserializeOwned,
{
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, HotreloadError> {
        // Get path of the config file containing directory
        let path = path.as_ref().to_path_buf();
        let watch_path = path.parent().ok_or(HotreloadError::NoParent)?.to_path_buf();

        // Init config type
        let config = Arc::new(Self::init(&path)?);
        let config_clone = config.clone();

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
            _data: PhantomData,
        })
    }

    pub fn config(&self) -> &Arc<C> {
        &self.config
    }

    fn init<P: AsRef<Path>>(path: P) -> Result<C, HotreloadError> {
        let data = Self::load_data(path)?;
        let config = C::default();
        config.apply(data);
        Ok(config)
    }

    fn reload<P: AsRef<Path>>(config: &C, path: P) -> Result<(), HotreloadError> {
        config.apply(Self::load_data(path)?);
        Ok(())
    }

    fn load_data<P: AsRef<Path>>(path: P) -> Result<D, HotreloadError> {
        // Open file
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) => {
                return Err(match error.kind() {
                    ErrorKind::NotFound => HotreloadError::NotFound(error),
                    ErrorKind::PermissionDenied => HotreloadError::PermissionDenied(error),
                    _ => HotreloadError::Io(error),
                })
            }
        };

        // Read content
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)
            .map_err(HotreloadError::FileRead)?;

        // Deserialize
        toml::from_str(&buffer).map_err(HotreloadError::Deserialize)
    }
}

#[derive(Debug, Error)]
pub enum HotreloadError {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestConfig {
        number: std::sync::Mutex<i32>,
    }

    struct TestData {
        number: i32,
    }

    impl HotreloadApply<TestData> for TestConfig {
        fn apply(&self, data: TestData) {
            *self.number.lock().unwrap() = data.number;
        }
    }
}
