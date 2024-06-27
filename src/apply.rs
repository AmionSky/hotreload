pub type ApplyError = Box<dyn std::error::Error>;

pub type ApplyResult = Result<(), ApplyError>;

pub trait Apply<D> {
    fn apply(&self, data: D) -> ApplyResult;
}

// Validity check
#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;

    struct TestConfig {
        number: std::sync::Mutex<i32>,
    }

    struct TestData {
        number: i32,
    }

    impl Apply<TestData> for TestConfig {
        fn apply(&self, data: TestData) -> ApplyResult {
            *self.number.lock().unwrap() = data.number;
            Ok(())
        }
    }
}
