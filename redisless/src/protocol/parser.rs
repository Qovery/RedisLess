use super::error::RedisError;
use super::{Resp, Result};
use super::{CR, LF, NIL_VALUE_SIZE};

pub struct RedisProtocolParser;

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
        RedisProtocolParser::parse_everything_until_crlf(input).map(|(x, y)| (Resp::String(x), y))
    }

    pub fn parse_errors(input: &[u8]) -> Result {
        RedisProtocolParser::parse_everything_until_crlf(input).map(|(x, y)| (Resp::Error(x), y))
    }

    pub fn parse_integers(input: &[u8]) -> Result {
        RedisProtocolParser::parse_everything_until_crlf(input).map(|(x, y)| (Resp::Integer(x), y))
    }

    pub fn parse_bulk_strings(input: &[u8]) -> Result {
        // Check Null Strings.
        if RedisProtocolParser::check_null_value(input) {
            Ok((Resp::Nil, &input[NIL_VALUE_SIZE..]))
        } else {
            let (size_str, input_after_size) =
                RedisProtocolParser::parse_everything_until_crlf(input)?;
            let size = std::str::from_utf8(size_str)?.parse::<u64>()? as usize;
            if RedisProtocolParser::check_crlf_at_index(input_after_size, size) {
                Ok((
                    Resp::BulkString(&input_after_size[..size]),
                    &input_after_size[size + 2..],
                ))
            } else {
                Err(RedisError::incorrect_format())
            }
        }
    }

    fn check_crlf_at_index(input: &[u8], index: usize) -> bool {
        input.len() >= index && input[index] == CR && input[index + 1] == LF
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
        Ok((Resp::Array(result), left))
    }
}
