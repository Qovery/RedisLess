pub mod in_memory;

pub trait Storage {
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn expire(&mut self, key: &[u8], duration: u64) -> u32;
    fn read(&mut self, key: &[u8]) -> Option<&[u8]>;
    fn remove(&mut self, key: &[u8]) -> u32;
    fn ttl(&mut self, key: &[u8]) -> i64;
}
