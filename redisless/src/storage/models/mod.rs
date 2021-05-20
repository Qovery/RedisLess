pub mod expiry;
pub mod hash;
pub mod meta;

// re-export so one can use with models::Expiry
// rather than models::expiry::Expiry
pub use expiry::Expiry;
pub use hash::RedisHashMap;
pub use meta::RedisMeta;

pub type RedisString = Vec<u8>;

pub enum RedisType {
    String,
    List,
    Set,
    Hash,
}
