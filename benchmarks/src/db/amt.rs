use crate::{db::AuthDB, run::CounterTrait};
use amt_db::{
    crypto::TypeDepths,
    simple_db::{new_simple_db, SimpleDb, INC_KEY_COUNT, INC_KEY_LEVEL_SUM, INC_TREE_COUNT},
    storage::access::PUT_COUNT,
    ver_tree::Key,
};

pub fn new(dir: &str) -> SimpleDb {
    new_simple_db::<TypeDepths>(dir, true).0
}

impl AuthDB for SimpleDb {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        self.get(&Key(key)).unwrap()
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.set(&Key(key), value.into_boxed_slice())
    }

    fn commit(&mut self, index: usize) {
        let _ = self.commit(index as u64).unwrap();
    }
}

#[derive(Clone)]
pub struct AMTCounter {
    put_count: [u64; 4],
    inc_key_count: u64,
    inc_tree_count: u64,
    inc_key_level_count: u64,
}

impl Default for AMTCounter {
    fn default() -> Self {
        Self {
            put_count: [0; 4],
            inc_key_count: 0,
            inc_tree_count: 0,
            inc_key_level_count: 0,
        }
    }
}

impl CounterTrait for AMTCounter {
    fn report(&mut self) -> String {
        let put_count = *PUT_COUNT.lock().unwrap();
        let inc_key_count = *INC_KEY_COUNT.lock().unwrap();
        let inc_tree_count = *INC_TREE_COUNT.lock().unwrap();
        let inc_key_level_count = *INC_KEY_LEVEL_SUM.lock().unwrap();

        let key_diff = inc_key_count - self.inc_key_count;
        let tree_diff = inc_tree_count - self.inc_tree_count;
        let level_diff = inc_key_level_count - self.inc_key_level_count;
        let avg_level = (level_diff as f64) / (key_diff as f64);

        let answer = format!(
            "avg levels: {:.3}, access writes {:?}, data writes {} {}",
            avg_level,
            self.put_count
                .iter()
                .zip(put_count.iter())
                .map(|(x, y)| y - x)
                .collect::<Vec<u64>>(),
            key_diff * 2,
            tree_diff * 2,
        );

        self.put_count = *PUT_COUNT.lock().unwrap();
        self.inc_key_count = *INC_KEY_COUNT.lock().unwrap();
        self.inc_tree_count = *INC_TREE_COUNT.lock().unwrap();
        self.inc_key_level_count = *INC_KEY_LEVEL_SUM.lock().unwrap();

        answer
    }
}
