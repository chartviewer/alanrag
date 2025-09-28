use crate::chunker::Chunk;
use anyhow::Result;
use std::path::Path;
use sha2::{Sha256, Digest};
use std::collections::HashMap;

pub struct ChunkStore {
    db: sled::Db,
    hash_index: sled::Db,  // Maps content hash to chunk ID for deduplication
}

impl ChunkStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let chunks_dir = data_dir.join("chunks");
        std::fs::create_dir_all(&chunks_dir)?;

        let db = sled::open(&chunks_dir)?;
        let hash_index = sled::open(chunks_dir.join("hash_index"))?;

        Ok(Self { db, hash_index })
    }

    pub fn store(&self, chunk: &Chunk) -> Result<bool> {
        // Generate content hash for deduplication
        let content_hash = self.generate_content_hash(&chunk.content);

        // Check if chunk already exists
        if let Some(existing_id) = self.hash_index.get(&content_hash)? {
            eprintln!("Duplicate chunk detected. Content hash: {}, existing ID: {}",
                     hex::encode(&content_hash),
                     String::from_utf8_lossy(&existing_id));
            return Ok(false);  // Duplicate, not stored
        }

        // Store the chunk
        let data = serde_json::to_vec(chunk)?;
        self.db.insert(&chunk.id, data)?;

        // Store the hash mapping
        self.hash_index.insert(content_hash, chunk.id.as_bytes())?;

        self.db.flush()?;
        self.hash_index.flush()?;

        Ok(true)  // New chunk stored
    }

    fn generate_content_hash(&self, content: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hasher.finalize().to_vec()
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
        self.hash_index.clear()?;
        self.db.flush()?;
        self.hash_index.flush()?;
        Ok(())
    }

    pub fn get_deduplication_stats(&self) -> Result<HashMap<String, usize>> {
        let mut stats = HashMap::new();
        stats.insert("total_chunks".to_string(), self.db.len());
        stats.insert("unique_content_hashes".to_string(), self.hash_index.len());
        Ok(stats)
    }
}