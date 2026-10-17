use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    /// Problem in the model definition (missing parameter, bad connection, ...).
    Model(String),
    /// Fluid property table problem.
    Fluid(String),
    /// Solver failure (step size underflow, Newton divergence, ...).
    Solver(String),
    /// Input parsing (YAML / JSON / units).
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Model(s) => write!(f, "model error: {s}"),
            Error::Fluid(s) => write!(f, "fluid error: {s}"),
            Error::Solver(s) => write!(f, "solver error: {s}"),
            Error::Parse(s) => write!(f, "parse error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
