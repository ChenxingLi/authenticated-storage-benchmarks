pub mod read_then_write;

pub use read_then_write::ReadThenWrite;

type Key = Vec<u8>;
type Value = Vec<u8>;

pub trait TaskTrait: Iterator<Item = Events> {
    fn warmup(&mut self);
}

pub enum Event {
    Read(Key),
    Write(Key, Value),
}

pub struct Events(pub Vec<Event>);

fn hash(input: &[u8]) -> [u8; 32] {
    let mut hasher = crc64fast::Digest::new();
    hasher.write(input);
    let checksum = hasher.sum64();
    unsafe { std::mem::transmute::<[u64; 4], [u8; 32]>([checksum; 4]) }
}
