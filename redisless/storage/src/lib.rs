pub mod in_memory;

pub trait Storage {
    fn set(&mut self, key: &[u8], value: &[u8]);
    fn get(&mut self, key: &[u8]) -> Option<&[u8]>;
    fn del(&mut self, key: &[u8]) -> u32;
    fn expire(&mut self, key: &[u8], value: &[u8]) -> u32;
}
