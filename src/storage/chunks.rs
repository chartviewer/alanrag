use crate::chunker::Chunk;
use anyhow::Result;
use std::path::Path;

pub struct ChunkStore {
    db: sled::Db,
}

impl ChunkStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let chunks_dir = data_dir.join("chunks");
        std::fs::create_dir_all(&chunks_dir)?;
        let db = sled::open(chunks_dir)?;

        Ok(Self { db })
    }

    pub fn store(&self, chunk: &Chunk) -> Result<()> {
        let data = serde_json::to_vec(chunk)?;
        self.db.insert(&chunk.id, data)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get(&self, chunk_id: &str) -> Result<Option<Chunk>> {
        if let Some(data) = self.db.get(chunk_id)? {
            let chunk: Chunk = serde_json::from_slice(&data)?;
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }

    pub fn delete(&self, chunk_id: &str) -> Result<()> {
        self.db.remove(chunk_id)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<String>> {
        let mut chunk_ids = Vec::new();

        for item in self.db.iter() {
            let (key, _) = item?;
            chunk_ids.push(String::from_utf8_lossy(&key).to_string());
        }

        Ok(chunk_ids)
    }

    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        self.db.flush()?;
        Ok(())
    }
}