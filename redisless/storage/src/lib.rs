pub mod in_memory;

pub trait Storage {
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn setex(&mut self, key: &[u8], value: &[u8], expiry: usize);
    fn get(&self, key: &[u8]) -> Option<&[u8]>;
    fn del(&mut self, key: &[u8]) -> u32;
}
