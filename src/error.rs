use std::fmt;

#[derive(Debug)]
pub struct RefSynError {
    pub message: String,
}

impl fmt::Display for RefSynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RefSynError: {}", self.message)
    }
}
