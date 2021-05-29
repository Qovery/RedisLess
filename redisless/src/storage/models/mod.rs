pub mod expiry;
pub mod hash;
pub mod zeta;

// re-export so one can use with models::Expiry
// rather than models::expiry::Expiry
pub use expiry::Expiry;
pub use hash::RedisHashMap;
pub use zeta::RedisMeta;

pub type RedisString = Vec<u8>;

pub enum RedisType {
    String,
    List,
    Set,
    Hash,
}
