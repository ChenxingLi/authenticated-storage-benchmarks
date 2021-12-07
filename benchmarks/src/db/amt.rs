use crate::opts::Options;
use crate::{db::AuthDB, run::CounterTrait};
use amt_db::crypto::export::ProjectiveCurve;
use amt_db::{
    amt_db::{cached_pp, AmtDb, INC_KEY_COUNT, INC_KEY_LEVEL_SUM, INC_TREE_COUNT},
    multi_layer_amt::Key,
    storage::access::PUT_COUNT,
};
use kvdb::KeyValueDB;
use std::sync::Arc;

pub struct AMTDB {
    amt: AmtDb,
    print_root_period: Option<usize>,
}

pub fn new(backend: Arc<dyn KeyValueDB>, opts: &Options) -> AMTDB {
    let pp = cached_pp("./pp");
    AMTDB {
        amt: AmtDb::new(backend, pp, true),
        print_root_period: if opts.print_root {
            Some(opts.report_epoch)
        } else {
            None
        },
    }
}

impl AuthDB for AMTDB {
    fn get(&self, key: Vec<u8>) -> Option<Box<[u8]>> {
        // println!("read");
        self.amt.get(&Key(key)).unwrap()
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        // println!("write");
        self.amt.set(&Key(key), value.into_boxed_slice())
    }

    fn commit(&mut self, index: usize) {
        // println!("commit");
        let (commit, root) = self.amt.commit(index as u64).unwrap();
        if let Some(period) = self.print_root_period {
            if index % period == 0 {
                let aff_comm = commit.into_affine();
                println!("Commitment {:?}, Merkle {:?}", aff_comm, root);
            }
        }
    }

    fn backend(&self) -> &dyn KeyValueDB {
        &*self.amt.kvdb
    }

    fn flush_all(&mut self) {
        self.amt.flush_root();
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
