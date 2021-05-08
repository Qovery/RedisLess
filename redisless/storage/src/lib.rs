pub mod in_memory;

pub trait Storage {
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn expire(&mut self, key: &[u8], duration: u64) -> u32;
    fn setex(&mut self, key: &[u8], value: &[u8], duration: u64);
    fn get(&mut self, key: &[u8]) -> Option<&[u8]>;
    fn ttl(&mut self, key: &[u8]) -> i64;
    fn del(&mut self, key: &[u8]) -> u32;
}
