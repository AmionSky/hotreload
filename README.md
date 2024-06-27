# hotreload

A simple crate to hotreload toml config files.

## Usage

```rust
use hotreload::{Hotreload, Apply};

#[derive(Default)]
struct Config {
    value: Mutex<i32>
}

#[derive(serde::Deserialize)]
struct Data {
    value: i32
}

impl Apply<Data> for Config {
    fn apply(&self, data: Data) -> hotreload::ApplyResult {
        *self.value.lock().unwrap() = data.value;
        Ok(())
    }
}

fn example() -> Result<(), hotreload::Error> {
    let watcher = Hotreload::<Config, Data>::new("my-config.toml")?;
    let config: Arc<Config> = watcher.config().clone()
}
```

## License

Licensed under either of

 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
