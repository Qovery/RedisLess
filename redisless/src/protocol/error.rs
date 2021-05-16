#[derive(Debug)]
pub struct RedisError {
    pub err_type: RedisErrorType,
}
#[derive(Debug)]
pub enum RedisErrorType {
    // Unknown symbol at index
    UnknownSymbol,
    // Attempting to parse an empty input
    EmptyInput,
    // Cannot find CRLF at index
    NoCrlf,
    // Incorrect format detected
    IncorrectFormat,
    Other(Box<dyn std::error::Error>),
}

impl RedisError {
    pub fn unknown_symbol() -> Self {
        Self {
            err_type: RedisErrorType::UnknownSymbol,
        }
    }

    pub fn empty_input() -> Self {
        Self {
            err_type: RedisErrorType::EmptyInput,
        }
    }

    pub fn no_crlf() -> Self {
        Self {
            err_type: RedisErrorType::NoCrlf,
        }
    }
    pub fn incorrect_format() -> Self {
        Self {
            err_type: RedisErrorType::IncorrectFormat,
        }
    }
}

impl<'a> std::fmt::Display for RedisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'a> std::error::Error for RedisError {}

impl<'a> From<std::str::Utf8Error> for RedisError {
    fn from(from: std::str::Utf8Error) -> Self {
        Self {
            err_type: RedisErrorType::Other(Box::new(from)),
        }
    }
}

impl<'a> From<std::num::ParseIntError> for RedisError {
    fn from(from: std::num::ParseIntError) -> Self {
        Self {
            err_type: RedisErrorType::Other(Box::new(from)),
        }
    }
}
