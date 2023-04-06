pub mod read_then_write;
pub mod real_trace;

pub use read_then_write::ReadThenWrite;
pub use real_trace::RealTrace;

type Key = Vec<u8>;
type Value = Vec<u8>;

pub trait TaskTrait {
    fn warmup<'a>(&'a self) -> Box<dyn Iterator<Item = Events> + 'a> {
        Box::new(NoopIter)
    }
    fn tasks<'a>(&'a self) -> Box<dyn Iterator<Item = Events> + 'a>;
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

pub struct NoopIter;

impl Iterator for NoopIter {
    type Item = Events;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
