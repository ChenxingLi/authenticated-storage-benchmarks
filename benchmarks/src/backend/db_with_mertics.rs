use kvdb::KeyValueDB;
use kvdb_rocksdb::Database;
use std::io::Read;

// Database with enabled statistics
pub struct DatabaseWithMetrics {
    db: Arc<Database>,
    pub reads: std::sync::atomic::AtomicI64,
    pub writes: std::sync::atomic::AtomicI64,
    bytes_read: std::sync::atomic::AtomicI64,
    bytes_written: std::sync::atomic::AtomicI64,
}

impl DatabaseWithMetrics {
    /// Create a new instance
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            reads: std::sync::atomic::AtomicI64::new(0),
            writes: std::sync::atomic::AtomicI64::new(0),
            bytes_read: std::sync::atomic::AtomicI64::new(0),
            bytes_written: std::sync::atomic::AtomicI64::new(0),
        }
    }
}

use convert::convert_tx;
use std::sync::Arc;

mod convert {
    use kvdb::{DBOp, DBTransaction};
    use smallvec::SmallVec;

    fn convert_op(op: kvdb01::DBOp) -> DBOp {
        match op {
            kvdb01::DBOp::Insert { col, key, value } => DBOp::Insert {
                col: col.unwrap_or(0),
                key: SmallVec::from_vec(key.into_vec()),
                value: value.into_vec(),
            },
            kvdb01::DBOp::Delete { col, key } => DBOp::Delete {
                col: col.unwrap_or(0),
                key: SmallVec::from_vec(key.into_vec()),
            },
        }
    }

    pub fn convert_tx(mut tx: kvdb01::DBTransaction) -> DBTransaction {
        let mut ops = Vec::with_capacity(tx.ops.len());
        for op in tx.ops.drain(..) {
            ops.push(convert_op(op));
        }
        DBTransaction { ops }
    }
}

impl kvdb01::KeyValueDB for DatabaseWithMetrics {
    fn get(&self, col: Option<u32>, key: &[u8]) -> std::io::Result<Option<kvdb01::DBValue>> {
        let col = col.unwrap_or(0);
        let res = self.db.get(col, key);
        let count = res
            .as_ref()
            .map_or(0, |y| y.as_ref().map_or(0, |x| x.bytes().count()));

        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.bytes_read
            .fetch_add(count as i64, std::sync::atomic::Ordering::Relaxed);

        Ok(res?.map(|x| kvdb01::DBValue::from_vec(x)))
    }
    fn get_by_prefix(&self, col: Option<u32>, prefix: &[u8]) -> Option<Box<[u8]>> {
        let col = col.unwrap_or(0);

        let res = self.db.get_by_prefix(col, prefix);
        let count = res.as_ref().map_or(0, |x| x.bytes().count());

        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.bytes_read
            .fetch_add(count as i64, std::sync::atomic::Ordering::Relaxed);

        res
    }
    fn write_buffered(&self, transaction: kvdb01::DBTransaction) {
        let mut count = 0;
        let transaction = convert_tx(transaction);
        for op in &transaction.ops {
            count += match op {
                kvdb::DBOp::Insert { value, .. } => value.bytes().count(),
                _ => 0,
            };
        }

        self.writes.fetch_add(
            transaction.ops.len() as i64,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.bytes_written
            .fetch_add(count as i64, std::sync::atomic::Ordering::Relaxed);

        self.db.write_buffered(transaction)
    }
    fn write(&self, transaction: kvdb01::DBTransaction) -> std::io::Result<()> {
        let mut count = 0;
        let transaction = convert_tx(transaction);
        for op in &transaction.ops {
            count += match op {
                kvdb::DBOp::Insert { value, .. } => value.bytes().count(),
                _ => 0,
            };
        }

        self.bytes_written
            .fetch_add(count as i64, std::sync::atomic::Ordering::Relaxed);
        self.writes.fetch_add(
            transaction.ops.len() as i64,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.db.write(transaction)
    }
    fn flush(&self) -> std::io::Result<()> {
        self.db.flush()
    }

    fn iter<'a>(
        &'a self,
        col: Option<u32>,
    ) -> Box<(dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a)> {
        let col = col.unwrap_or(0);
        kvdb::KeyValueDB::iter(&*self.db, col)
    }

    fn iter_from_prefix<'a>(
        &'a self,
        col: Option<u32>,
        prefix: &'a [u8],
    ) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        let col = col.unwrap_or(0);
        self.db.iter_from_prefix(col, prefix)
    }

    fn restore(&self, new_db: &str) -> std::io::Result<()> {
        self.db.restore(new_db)
    }
}

impl parity_journaldb::KeyValueDB for DatabaseWithMetrics {}

impl stats::PrometheusMetrics for DatabaseWithMetrics {
    fn prometheus_metrics(&self, p: &mut stats::PrometheusRegistry) {
        p.register_counter(
            "kvdb_reads",
            "db reads",
            self.reads.load(std::sync::atomic::Ordering::Relaxed) as i64,
        );
        p.register_counter(
            "kvdb_writes",
            "db writes",
            self.writes.load(std::sync::atomic::Ordering::Relaxed) as i64,
        );
        p.register_counter(
            "kvdb_bytes_read",
            "db bytes_reads",
            self.bytes_read.load(std::sync::atomic::Ordering::Relaxed) as i64,
        );
        p.register_counter(
            "kvdb_bytes_written",
            "db bytes_written",
            self.bytes_written
                .load(std::sync::atomic::Ordering::Relaxed) as i64,
        );
    }
}
