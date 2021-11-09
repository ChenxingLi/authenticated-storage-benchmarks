// The original journaldb relies on some metric run in crate `ethcore-db`. But it doesn't rely on the other dependencies.

pub use self::memory_db::InMemoryWithMetrics;

mod memory_db {
    // Copyright 2015-2020 Parity Technologies (UK) Ltd.
    // The following code is part of OpenEthereum.

    // OpenEthereum is free software: you can redistribute it and/or modify
    // it under the terms of the GNU General Public License as published by
    // the Free Software Foundation, either version 3 of the License, or
    // (at your option) any later version.

    // OpenEthereum is distributed in the hope that it will be useful,
    // but WITHOUT ANY WARRANTY; without even the implied warranty of
    // MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    // GNU General Public License for more details.

    // You should have received a copy of the GNU General Public License
    // along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

    /// InMemory with disabled statistics
    pub struct InMemoryWithMetrics {
        db: kvdb_memorydb::InMemory,
    }

    impl kvdb01::KeyValueDB for InMemoryWithMetrics {
        fn get(&self, col: Option<u32>, key: &[u8]) -> std::io::Result<Option<kvdb01::DBValue>> {
            self.db.get(col, key)
        }
        fn get_by_prefix(&self, col: Option<u32>, prefix: &[u8]) -> Option<Box<[u8]>> {
            self.db.get_by_prefix(col, prefix)
        }
        fn write_buffered(&self, transaction: kvdb01::DBTransaction) {
            self.db.write_buffered(transaction)
        }
        fn write(&self, transaction: kvdb01::DBTransaction) -> std::io::Result<()> {
            self.db.write(transaction)
        }
        fn flush(&self) -> std::io::Result<()> {
            self.db.flush()
        }

        fn iter<'a>(
            &'a self,
            col: Option<u32>,
        ) -> Box<(dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a)> {
            kvdb01::KeyValueDB::iter(&self.db, col)
        }

        fn iter_from_prefix<'a>(
            &'a self,
            _col: Option<u32>,
            _prefix: &'a [u8],
        ) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
            unimplemented!();
            // self.db.iter_from_prefix(col, prefix)
        }

        fn restore(&self, new_db: &str) -> std::io::Result<()> {
            self.db.restore(new_db)
        }
    }

    impl stats::PrometheusMetrics for InMemoryWithMetrics {
        fn prometheus_metrics(&self, _: &mut stats::PrometheusRegistry) {}
    }

    impl parity_journaldb::KeyValueDB for InMemoryWithMetrics {}

    impl InMemoryWithMetrics {
        /// Create new instance
        pub fn create(num_cols: u32) -> Self {
            Self {
                db: kvdb_memorydb::create(num_cols),
            }
        }
    }
}
