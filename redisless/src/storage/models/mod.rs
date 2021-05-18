pub mod meta;
pub mod expiry;
pub mod hash;

// re-export so one can use with models::Expiry
// rather than models::expiry::Expiry
pub use meta::RedisMeta;
pub use expiry::Expiry;
pub use hash::RedisHashMap;

pub type RedisString = Vec<u8>;

pub enum RedisType {
    String,
    List,
    Set,
    Hash,
}
