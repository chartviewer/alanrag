#!/bin/bash
# Demo script for RAG MCP Server

echo "🚀 RAG MCP Server Demo"
echo "======================"
echo

# Make sure we're in the right directory
cd "$(dirname "$0")/.."

# Build the project if needed
if [ ! -f "target/release/rag-mcp-server" ]; then
    echo "📦 Building the project..."
    cargo build --release
    echo
fi

echo "📄 Step 1: Ingesting test document..."
python3 scripts/ingest.py test_document.md
echo

echo "🔍 Step 2: Searching for 'machine learning algorithms'..."
python3 scripts/query.py "machine learning algorithms" --top-k 3
echo

echo "🔍 Step 3: Searching for 'neural networks'..."
python3 scripts/query.py "neural networks" --top-k 2
echo

echo "🔍 Step 4: Searching for 'rust programming'..."
python3 scripts/query.py "rust programming" --top-k 2
echo

echo "✅ Demo completed!"
echo
echo "💡 Try these commands yourself:"
echo "  python3 scripts/ingest.py <your_document>"
echo "  python3 scripts/query.py '<your_query>'"
echo "  python3 scripts/query.py '<your_query>' --type chapters"