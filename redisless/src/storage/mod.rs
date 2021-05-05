pub mod in_memory;

pub trait Storage {
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn get(&self, key: &[u8]) -> Option<&[u8]>;
    fn del(&mut self, key: &[u8]) -> u32;
}
