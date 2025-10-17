pub mod embeddings;
pub mod chunks;
pub mod index;
pub mod sqlite_storage;

// Export both implementations
pub use index::Storage as SledStorage;
pub use sqlite_storage::SqliteStorage;

// Default to SQLite for multi-process support
pub use sqlite_storage::SqliteStorage as Storage;
pub use sqlite_storage::SearchResult;