use in_memory::Expiry;

pub mod in_memory;

pub trait Storage {
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn expire(&mut self, key: &[u8], expiry: Expiry) -> u32;
    fn read(&mut self, key: &[u8]) -> Option<&[u8]>;
    fn remove(&mut self, key: &[u8]) -> u32;
    fn value_type(&mut self, key: &[u8]) -> &str;
    fn contains(&mut self, key: &[u8]) -> bool;
}
