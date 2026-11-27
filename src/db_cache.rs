//! Database caching for tile downloads
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
pub struct TileCacheDB {
    db_path: PathBuf,
    entries: HashMap<(u8, i32, i32), Vec<u8>>,
    cache_dir: Option<std::path::PathBuf>,
}

impl TileCacheDB {
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            db_path: cache_root.join("tile_cache.db"),
            entries: HashMap::new(),
            cache_dir: Some(cache_root),
        }
    }

    pub fn get(&self, z: u8, x: i32, y: i32) -> Option<Vec<u8>> {
        self.entries.get(&(z, x, y)).cloned()
    }

    pub fn store(&mut self, z: u8, x: i32, y: i32, data: Vec<u8>) {
        self.entries.insert((z, x, y), data);
    }

    pub fn flush(&self) -> std::io::Result<()> {
        // Would implement SQLite or similar
        Ok(())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
