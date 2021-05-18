pub mod meta;
pub mod expiry;
pub mod string;
pub mod hash;

// re-export so one can use with models::Expiry
// rather than models::expiry::Expiry
pub use meta::RedisMeta;
pub use expiry::{Expiry, Expire};
pub use string::RedisString;
pub use hash::RedisHashMap;

pub type RedisValue = Vec<u8>;
pub enum RedisType {
    String,
    List,
    Set,
    Hash,
}
