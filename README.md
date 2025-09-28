# RAG MCP Server

An intelligent Retrieval-Augmented Generation (RAG) system implemented as a Model Context Protocol (MCP) server in Rust.

## Features

- **Semantic Document Chunking**: Intelligently splits documents (PDF, Markdown, text, source code) into meaningful chunks
- **Graph Relationships**: Builds relationships between words, chunks, and chapters for enhanced retrieval
- **Local Storage**: Uses embedded databases (sled) for data persistence without network dependencies
- **Vector Embeddings**: Generates embeddings for semantic search (placeholder implementation - ready for production models)
- **MCP Protocol**: Implements JSON-RPC over stdin/stdout for easy integration with LLM applications
- **Easy-to-use Scripts**: Python scripts for ingesting documents and querying the knowledge base

## Supported MCP Tools

1. **`ingest`** - Process and store documents
   - Parameters: `path` (file path), `doc_type` (optional: pdf, markdown, text, code)
   - Chunks documents and stores them with embeddings

2. **`search_knowledge_chunk`** - Search for relevant document chunks
   - Parameters: `query` (search text), `top_k` (optional: number of results, default 10)
   - Returns ranked chunks with similarity scores

3. **`search_knowledge_chapter`** - Search for relevant chapters/sections
   - Parameters: `query` (search text), `top_k` (optional: number of results, default 5)
   - Returns complete chapters/sections containing relevant content

## Installation

```bash
# Clone and build
git clone <repo>
cd rag-mcp-server
cargo build --release
```

## Quick Start

### 1. Ingest Documents

```bash
# Ingest a PDF document
python scripts/ingest.py documents/sample.pdf

# Ingest markdown with explicit type
python scripts/ingest.py documents/readme.md markdown

# Ingest source code
python scripts/ingest.py src/main.rs

# Using shell wrapper
./scripts/ingest.sh documents/article.txt
```

### 2. Query Knowledge Base

```bash
# Search for chunks
python scripts/query.py "machine learning algorithms"

# Search for chapters with specific parameters
python scripts/query.py "neural networks" --type chapters --top-k 5

# Using shell wrapper
./scripts/query.sh "rust programming" --type chunks --top-k 10
```

### 3. Direct MCP Usage

You can also run the server directly and communicate via JSON-RPC:

```bash
# Start the server (reads from stdin, writes to stdout)
cargo run --release

# Send JSON-RPC requests via stdin
echo '{"jsonrpc":"2.0","method":"ingest","params":["./README.md","markdown"],"id":1}' | cargo run --release
```

## Configuration

Edit `alan_config.yaml` to customize:

```yaml
storage:
  data_dir: "./data"           # Local storage directory
  max_chunk_size: 512          # Maximum tokens per chunk
  min_chunk_size: 100          # Minimum tokens per chunk

chunking:
  overlap_tokens: 50           # Overlap between chunks
  semantic_threshold: 0.75     # Semantic similarity threshold
  code_languages:              # Supported programming languages
    - rust
    - python
    - javascript

embedding:
  model_name: "sentence-transformers/all-MiniLM-L6-v2"
  dimension: 384               # Embedding vector dimension
  batch_size: 32              # Batch size for embedding generation

mcp:
  transport: "stdio"          # Uses stdin/stdout for communication

graph:
  max_connections: 10         # Max graph connections per node
  similarity_threshold: 0.7   # Graph edge threshold
```

## Script Usage Examples

### Ingest Script

```bash
# Basic usage (auto-detects file type)
python scripts/ingest.py README.md

# Specify document type explicitly
python scripts/ingest.py documents/paper.pdf pdf

# Ingest source code
python scripts/ingest.py src/main.rs code

# Get help
python scripts/ingest.py --help
```

### Query Script

```bash
# Basic search for chunks
python scripts/query.py "machine learning"

# Search for chapters
python scripts/query.py "neural networks" --type chapters

# Limit results
python scripts/query.py "rust programming" --top-k 5

# Combined options
python scripts/query.py "semantic search" --type chunks --top-k 10

# Get help
python scripts/query.py --help
```

### Raw JSON-RPC Examples

If you prefer to use the server directly:

```bash
# Ingest a document
echo '{"jsonrpc":"2.0","method":"ingest","params":["./README.md","markdown"],"id":1}' | \
  ./target/release/rag-mcp-server

# Search for chunks
echo '{"jsonrpc":"2.0","method":"search_knowledge_chunk","params":["machine learning",5],"id":2}' | \
  ./target/release/rag-mcp-server

# Search for chapters
echo '{"jsonrpc":"2.0","method":"search_knowledge_chapter","params":["neural networks",3],"id":3}' | \
  ./target/release/rag-mcp-server
```

## Architecture

```
rag-mcp-server/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration management
│   ├── chunker/             # Document processing
│   │   ├── semantic.rs      # Semantic chunking logic
│   │   ├── pdf.rs          # PDF processing
│   │   ├── markdown.rs     # Markdown processing
│   │   └── code.rs         # Source code processing
│   ├── graph/               # Relationship graphs
│   │   ├── builder.rs      # Graph construction
│   │   └── relationships.rs # Graph analysis
│   ├── storage/             # Data persistence
│   │   ├── index.rs        # Search index
│   │   ├── embeddings.rs   # Embedding model
│   │   └── chunks.rs       # Chunk storage
│   ├── mcp/                 # MCP server
│   │   ├── server.rs       # JSON-RPC implementation
│   │   └── handlers.rs     # Request handlers
│   └── search/              # Search algorithms
│       ├── semantic.rs     # Semantic search
│       └── retrieval.rs    # Hybrid retrieval
└── data/                    # Local storage (created at runtime)
    ├── chunks/             # Document chunks
    ├── metadata/           # Chunk metadata
    └── embeddings/         # Vector embeddings
```

## Development

### Running Tests

```bash
cargo test
```

### Building for Production

```bash
cargo build --release
```

### Adding Real Embeddings

To use actual sentence transformers instead of the placeholder implementation:

1. Uncomment the candle dependencies in `Cargo.toml`
2. Update `src/storage/embeddings.rs` to use candle-transformers
3. Download models from Hugging Face Hub

## Performance Notes

- The current implementation uses a placeholder embedding model for development
- In production, consider using optimized vector databases like FAISS or Qdrant
- Graph relationships are stored in memory - consider disk-based storage for large datasets
- Semantic chunking can be enhanced with more sophisticated NLP models

## Demo

Run the demonstration script to see the system in action:

```bash
./scripts/demo.sh
```

This will:
1. Build the project (if needed)
2. Ingest the test document
3. Perform several example queries
4. Show the results

## Script Reference

### Ingest Script (`scripts/ingest.py`)

**Usage:** `python scripts/ingest.py <document_path> [document_type]`

**Auto-detected types:**
- `.pdf` → pdf
- `.md`, `.markdown` → markdown
- `.txt` → text
- `.rs`, `.py`, `.js`, `.ts`, `.java`, `.cpp`, `.c`, `.go` → code

**Examples:**
```bash
python scripts/ingest.py document.pdf
python scripts/ingest.py README.md markdown
python scripts/ingest.py src/main.rs
```

### Query Script (`scripts/query.py`)

**Usage:** `python scripts/query.py <query> [--type chunks|chapters] [--top-k N]`

**Parameters:**
- `--type`: Search type (chunks or chapters, default: chunks)
- `--top-k`: Maximum results (default: 10 for chunks, 5 for chapters)

**Examples:**
```bash
python scripts/query.py "machine learning"
python scripts/query.py "neural networks" --type chapters
python scripts/query.py "rust programming" --top-k 5
```

## System Status

✅ **Working Features:**
- Document ingestion (PDF, Markdown, text, code)
- Semantic chunking with configurable parameters
- Vector embeddings (placeholder implementation)
- Graph relationship building
- Chunk-based search with similarity scoring
- Text-based search as fallback
- stdio-based MCP protocol
- Python scripts for easy interaction

⚠️ **Placeholder Features (ready for production enhancement):**
- Embeddings use deterministic hash-based vectors
- Chapter search needs improved markdown parsing
- Graph traversal could be optimized

🚀 **Ready for Production:**
- Replace embedding model with real transformers
- Add advanced vector databases (FAISS, Qdrant)
- Enhance graph algorithms
- Add multi-modal document support

## License

[Add your license here]