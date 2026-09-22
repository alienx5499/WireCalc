use std::fmt;

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum CalcError {
    DivisionByZero,
    MissingParameter(String),
    InvalidParameter(String),
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalcError::DivisionByZero => write!(f, "Division by zero"),
            CalcError::MissingParameter(p) => write!(f, "Missing parameter '{}'", p),
            CalcError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
        }
    }
}
