use super::*;
use crate::protocol::{error::RedisErrorType, parser::RedisProtocolParser};

#[test]
pub fn test_simple_string() -> std::result::Result<(), RedisError> {
    let input = "+hello\r\n".as_bytes();
    let (resp, left) = RedisProtocolParser::parse(input)?;
    assert_eq!(resp, Resp::String("hello".as_bytes()));
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
    assert_eq!(resp, Resp::Nil);
    assert!(left.is_empty());
    Ok(())
}

#[test]
pub fn test_bulk_string() -> std::result::Result<(), RedisError> {
    let input = "$6\r\nfoobar\r\n".as_bytes();
    let (resp, left) = RedisProtocolParser::parse(input)?;
    assert_eq!(resp, Resp::BulkString("foobar".as_bytes()));
    assert!(left.is_empty());
    let input = "$0\r\n\r\n".as_bytes();
    let (resp, left) = RedisProtocolParser::parse(input)?;
    assert_eq!(resp, Resp::BulkString("".as_bytes()));
    assert!(left.is_empty());
    Ok(())
}

#[test]
pub fn test_arrays() -> std::result::Result<(), RedisError> {
    let input = "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".as_bytes();
    let (resp, left) = RedisProtocolParser::parse(input)?;
    assert_eq!(
        resp,
        Resp::Array(vec![
            Resp::BulkString("foo".as_bytes()),
            Resp::BulkString("bar".as_bytes())
        ])
    );
    assert!(left.is_empty());
    let input = "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n".as_bytes();
    let (resp, left) = RedisProtocolParser::parse(input)?;
    assert_eq!(
        resp,
        Resp::Array(vec![
            Resp::Integer("1".as_bytes()),
            Resp::Integer("2".as_bytes()),
            Resp::Integer("3".as_bytes()),
            Resp::Integer("4".as_bytes()),
            Resp::BulkString("foobar".as_bytes()),
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
        Resp::Array(vec![
            Resp::Array(vec![
                Resp::Integer("1".as_bytes()),
                Resp::Integer("2".as_bytes()),
                Resp::Integer("3".as_bytes()),
            ]),
            Resp::Array(vec![
                Resp::String("Foo".as_bytes()),
                Resp::Error("Bar".as_bytes()),
            ]),
        ])
    );
    assert!(left.is_empty());
    Ok(())
}
