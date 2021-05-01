pub type Result<'a> = std::result::Result<(RESP<'a>, &'a [u8]), RedisError>;

const NIL_VALUE_SIZE: usize = 4;
const CR: u8 = b'\r';
const LF: u8 = b'\n';

pub struct RedisProtocolParser;

#[derive(Debug, Eq, PartialEq)]
pub enum RESP<'a> {
    String(&'a [u8]),
    Error(&'a [u8]),
    Integer(&'a [u8]),
    BulkString(&'a [u8]),
    Array(Vec<RESP<'a>>),
    Nil,
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

#[derive(Debug)]
pub struct RedisError {
    err_type: RedisErrorType,
}

impl RedisError {
    fn unknown_symbol() -> Self {
        Self {
            err_type: RedisErrorType::UnknownSymbol,
        }
    }

    fn empty_input() -> Self {
        Self {
            err_type: RedisErrorType::EmptyInput,
        }
    }

    fn no_crlf() -> Self {
        Self {
            err_type: RedisErrorType::NoCrlf,
        }
    }
    fn incorrect_format() -> Self {
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

impl RedisProtocolParser {
    pub fn parse(input: &[u8]) -> Result {
        if let Some(first) = input.get(0) {
            let first = *first as char;
            let input = &input[1..];
            let (resp, left) = match first {
                '+' => RedisProtocolParser::parse_simple_string(input)?,
                ':' => RedisProtocolParser::parse_integers(input)?,
                '$' => RedisProtocolParser::parse_bulk_strings(input)?,
                '*' => RedisProtocolParser::parse_arrays(input)?,
                '-' => RedisProtocolParser::parse_errors(input)?,
                _ => return Err(RedisError::unknown_symbol()),
            };
            Ok((resp, left))
        } else {
            Err(RedisError::empty_input())
        }
    }

    fn parse_everything_until_crlf(
        input: &[u8],
    ) -> std::result::Result<(&[u8], &[u8]), RedisError> {
        for (index, (first, second)) in input.iter().zip(input.iter().skip(1)).enumerate() {
            if first == &CR && second == &LF {
                return Ok((&input[0..index], &input[index + 2..]));
            }
        }
        Err(RedisError::no_crlf())
    }

    pub fn parse_simple_string(input: &[u8]) -> Result {
        RedisProtocolParser::parse_everything_until_crlf(input).map(|(x, y)| (RESP::String(x), y))
    }

    pub fn parse_errors(input: &[u8]) -> Result {
        RedisProtocolParser::parse_everything_until_crlf(input).map(|(x, y)| (RESP::Error(x), y))
    }

    pub fn parse_integers(input: &[u8]) -> Result {
        RedisProtocolParser::parse_everything_until_crlf(input).map(|(x, y)| (RESP::Integer(x), y))
    }

    pub fn parse_bulk_strings(input: &[u8]) -> Result {
        // Check Null Strings.
        if RedisProtocolParser::check_null_value(input) {
            Ok((RESP::Nil, &input[NIL_VALUE_SIZE..]))
        } else {
            let (size_str, input_after_size) =
                RedisProtocolParser::parse_everything_until_crlf(input)?;
            let size = std::str::from_utf8(size_str)?.parse::<u64>()? as usize;
            if RedisProtocolParser::check_crlf_at_index(input_after_size, size) {
                Ok((
                    RESP::BulkString(&input_after_size[..size]),
                    &input_after_size[size + 2..],
                ))
            } else {
                Err(RedisError::incorrect_format())
            }
        }
    }

    fn check_crlf_at_index(input: &[u8], index: usize) -> bool {
        input.len() >= index && input[index] == b'\r' && input[index + 1] == b'\n'
    }

    fn check_null_value(input: &[u8]) -> bool {
        input.len() >= 4 && input[0] == b'-' && input[1] == b'1' && input[2] == CR && input[3] == LF
    }

    pub fn parse_arrays(input: &[u8]) -> Result {
        let (size_str, input) = RedisProtocolParser::parse_everything_until_crlf(input)?;
        let size = std::str::from_utf8(size_str)?.parse::<u64>()?;
        let sizes = size as usize;
        let mut left = input;
        let mut result = Vec::with_capacity(sizes);
        for _ in 0..sizes {
            let (element, tmp) = RedisProtocolParser::parse(left)?;
            result.push(element);
            left = tmp;
        }
        Ok((RESP::Array(result), left))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_simple_string() -> std::result::Result<(), RedisError> {
        let input = "+hello\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(resp, RESP::String("hello".as_bytes()));
        assert!(left.is_empty());
        Ok(())
    }

    #[test]
    pub fn test_errors() -> std::result::Result<(), RedisError> {
        let input = "+hello".as_bytes();
        let err = RedisProtocolParser::parse(input).unwrap_err();
        assert!(matches!(err.err_type, RedisErrorType::NoCrlf));
        let input = "*2\r\n$3\r\nfoo\r\n)hello".as_bytes();
        let err = RedisProtocolParser::parse(input).unwrap_err();
        assert!(matches!(err.err_type, RedisErrorType::UnknownSymbol));
        let input = "".as_bytes();
        let err = RedisProtocolParser::parse(input).unwrap_err();
        assert!(matches!(err.err_type, RedisErrorType::EmptyInput));
        let input = "$4\r\nfoo\r\n".as_bytes();
        let err = RedisProtocolParser::parse(input).unwrap_err();
        assert!(matches!(err.err_type, RedisErrorType::IncorrectFormat));
        let input = "*2\r\n$3\r\nfoo+hello\r\n".as_bytes();
        let err = RedisProtocolParser::parse(input).unwrap_err();
        assert!(matches!(err.err_type, RedisErrorType::IncorrectFormat));
        Ok(())
    }

    #[test]
    pub fn test_nil() -> std::result::Result<(), RedisError> {
        let input = "$-1\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(resp, RESP::Nil);
        assert!(left.is_empty());
        Ok(())
    }

    #[test]
    pub fn test_bulk_string() -> std::result::Result<(), RedisError> {
        let input = "$6\r\nfoobar\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(resp, RESP::BulkString("foobar".as_bytes()));
        assert!(left.is_empty());
        let input = "$0\r\n\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(resp, RESP::BulkString("".as_bytes()));
        assert!(left.is_empty());
        Ok(())
    }

    #[test]
    pub fn test_arrays() -> std::result::Result<(), RedisError> {
        let input = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(
            resp,
            RESP::Array(vec![
                RESP::BulkString("foo".as_bytes()),
                RESP::BulkString("bar".as_bytes())
            ])
        );
        assert!(left.is_empty());
        let input = "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(
            resp,
            RESP::Array(vec![
                RESP::Integer("1".as_bytes()),
                RESP::Integer("2".as_bytes()),
                RESP::Integer("3".as_bytes()),
                RESP::Integer("4".as_bytes()),
                RESP::BulkString("foobar".as_bytes()),
            ])
        );
        assert!(left.is_empty());
        Ok(())
    }

    #[test]
    pub fn test_array_of_arrays() -> std::result::Result<(), RedisError> {
        let input = "*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n".as_bytes();
        let (resp, left) = RedisProtocolParser::parse(input)?;
        assert_eq!(
            resp,
            RESP::Array(vec![
                RESP::Array(vec![
                    RESP::Integer("1".as_bytes()),
                    RESP::Integer("2".as_bytes()),
                    RESP::Integer("3".as_bytes()),
                ]),
                RESP::Array(vec![
                    RESP::String("Foo".as_bytes()),
                    RESP::Error("Bar".as_bytes()),
                ]),
            ])
        );
        assert!(left.is_empty());
        Ok(())
    }
}
