pub mod expiry;
pub mod redis_value;

// re-export so one can use with models::Expiry
// rather than models::expiry::Expiry
pub use expiry::Expiry;
pub use redis_value::RedisValue;

pub enum DataType {
    String,
    List,
    Set,
    Hash,
}
