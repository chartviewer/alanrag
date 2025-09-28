#!/usr/bin/env python3
"""
Query the RAG MCP server for relevant documents.

Usage:
    python scripts/query.py <query> [search_type] [top_k]

Search Types:
    chunks   - Search for relevant document chunks (default)
    chapters - Search for relevant chapters/sections

Examples:
    python scripts/query.py "machine learning algorithms"
    python scripts/query.py "neural networks" chunks 10
    python scripts/query.py "rust programming" chapters 5
"""

import json
import subprocess
import sys
import argparse
from pathlib import Path

def send_mcp_request(method, params, server_binary):
    """Send a JSON-RPC request to the MCP server via stdio."""
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    }

    request_json = json.dumps(request) + "\n"

    try:
        # Start the MCP server process
        process = subprocess.Popen(
            [server_binary],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            cwd=Path(server_binary).parent.parent  # Run from project root
        )

        # Send the request
        stdout, stderr = process.communicate(input=request_json, timeout=30)

        if process.returncode != 0:
            print(f"Error: Server process failed with return code {process.returncode}")
            if stderr:
                print(f"Stderr: {stderr}")
            return None

        # Parse the response
        if stdout.strip():
            try:
                response = json.loads(stdout.strip())
                return response
            except json.JSONDecodeError as e:
                print(f"Error parsing response: {e}")
                print(f"Raw output: {stdout}")
                return None
        else:
            print("No output from server")
            return None

    except subprocess.TimeoutExpired:
        process.kill()
        print("Error: Server request timed out")
        return None
    except Exception as e:
        print(f"Error communicating with server: {e}")
        return None

def display_chunk_results(results):
    """Display chunk search results in a formatted way."""
    chunks = results.get('chunks', [])
    total = results.get('total_found', len(chunks))

    print(f"\n🔍 Found {total} relevant chunks:")
    print("=" * 80)

    for i, chunk in enumerate(chunks, 1):
        score = chunk.get('score', 0.0)
        content = chunk.get('content', '')
        metadata = chunk.get('metadata', {})

        # Truncate content for display
        display_content = content[:200] + "..." if len(content) > 200 else content

        print(f"\n📄 Result {i} (Score: {score:.3f})")
        print(f"Source: {metadata.get('source_file', 'Unknown')}")
        if metadata.get('chapter'):
            print(f"Chapter: {metadata.get('chapter')}")
        if metadata.get('section'):
            print(f"Section: {metadata.get('section')}")
        if metadata.get('language'):
            print(f"Language: {metadata.get('language')}")

        print(f"\nContent:")
        print("-" * 40)
        print(display_content)
        print("-" * 40)

def display_chapter_results(results):
    """Display chapter search results in a formatted way."""
    chapters = results.get('chapters', [])
    total = results.get('total_found', len(chapters))

    print(f"\n📚 Found {total} relevant chapters:")
    print("=" * 80)

    for i, chapter in enumerate(chapters, 1):
        score = chapter.get('score', 0.0)
        chapter_name = chapter.get('chapter', 'Unknown')
        file_name = chapter.get('file', 'Unknown')
        chunk_count = chapter.get('chunk_count', 0)
        chunks = chapter.get('chunks', [])

        print(f"\n📖 Chapter {i} (Score: {score:.3f})")
        print(f"Name: {chapter_name}")
        print(f"File: {file_name}")
        print(f"Chunks: {chunk_count}")

        # Show first few chunks from the chapter
        print(f"\nPreview:")
        print("-" * 40)
        for j, chunk in enumerate(chunks[:2]):  # Show first 2 chunks
            content = chunk.get('content', '')
            preview = content[:150] + "..." if len(content) > 150 else content
            print(f"[Chunk {j+1}] {preview}")
            if j < len(chunks) - 1:
                print()

        if len(chunks) > 2:
            print(f"... and {len(chunks) - 2} more chunks")
        print("-" * 40)

def query_documents(query, search_type="chunks", top_k=10, server_binary=None):
    """Query the RAG system for relevant documents."""

    # Find the server binary
    if server_binary is None:
        script_dir = Path(__file__).parent
        server_binary = script_dir.parent / "target" / "release" / "rag-mcp-server"

        if not server_binary.exists():
            # Try debug build
            server_binary = script_dir.parent / "target" / "debug" / "rag-mcp-server"

        if not server_binary.exists():
            print("Error: Could not find rag-mcp-server binary. Please build the project first:")
            print("  cargo build --release")
            return False

    print(f"🔍 Searching for: '{query}'")
    print(f"Search type: {search_type}")
    print(f"Max results: {top_k}")
    print(f"Using server: {server_binary}")

    # Choose the appropriate method
    if search_type.lower() in ["chunks", "chunk"]:
        method = "search_knowledge_chunk"
    elif search_type.lower() in ["chapters", "chapter"]:
        method = "search_knowledge_chapter"
    else:
        print(f"Error: Unknown search type '{search_type}'. Use 'chunks' or 'chapters'.")
        return False

    # Send search request
    response = send_mcp_request(method, [query, top_k], str(server_binary))

    if response is None:
        print("Failed to get response from server")
        return False

    if "error" in response:
        print(f"Server error: {response['error']}")
        return False

    if "result" in response:
        result = response["result"]

        if search_type.lower() in ["chunks", "chunk"]:
            display_chunk_results(result)
        else:
            display_chapter_results(result)

        return True
    else:
        print("Unexpected response format")
        print(json.dumps(response, indent=2))
        return False

def main():
    parser = argparse.ArgumentParser(
        description="Query the RAG MCP server for relevant documents",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python scripts/query.py "machine learning algorithms"
  python scripts/query.py "neural networks" --type chunks --top-k 10
  python scripts/query.py "rust programming" --type chapters --top-k 5
        """
    )

    parser.add_argument("query", help="Search query")
    parser.add_argument(
        "--type", "-t",
        choices=["chunks", "chapters"],
        default="chunks",
        help="Search type: chunks or chapters (default: chunks)"
    )
    parser.add_argument(
        "--top-k", "-k",
        type=int,
        default=10,
        help="Maximum number of results to return (default: 10)"
    )
    parser.add_argument(
        "--server", "-s",
        help="Path to the rag-mcp-server binary (auto-detected if not specified)"
    )

    args = parser.parse_args()

    success = query_documents(
        query=args.query,
        search_type=args.type,
        top_k=args.top_k,
        server_binary=args.server
    )

    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()