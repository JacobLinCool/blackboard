use std::error::Error;
use std::fmt::{Display, Formatter};

pub type Res<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct AppErr(pub &'static str, pub String);

impl Display for AppErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.0, self.1)
    }
}

impl Error for AppErr {}
