#!/usr/bin/env python3
"""
Ingest documents into the RAG MCP server.

Usage:
    python scripts/ingest.py <document_path> [document_type]

Examples:
    python scripts/ingest.py documents/sample.pdf pdf
    python scripts/ingest.py documents/readme.md markdown
    python scripts/ingest.py src/main.rs code
    python scripts/ingest.py documents/article.txt text
"""

import json
import subprocess
import sys
import os
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

def detect_document_type(file_path):
    """Auto-detect document type from file extension."""
    suffix = Path(file_path).suffix.lower()

    type_map = {
        '.pdf': 'pdf',
        '.md': 'markdown',
        '.markdown': 'markdown',
        '.txt': 'text',
        '.rs': 'code',
        '.py': 'code',
        '.js': 'code',
        '.ts': 'code',
        '.java': 'code',
        '.cpp': 'code',
        '.c': 'code',
        '.go': 'code',
    }

    return type_map.get(suffix, 'text')

def ingest_document(document_path, document_type=None, server_binary=None):
    """Ingest a document into the RAG system."""

    # Validate document path
    if not os.path.exists(document_path):
        print(f"Error: Document not found: {document_path}")
        return False

    # Auto-detect document type if not provided
    if document_type is None:
        document_type = detect_document_type(document_path)
        print(f"Auto-detected document type: {document_type}")

    # Convert to absolute path
    abs_path = os.path.abspath(document_path)

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

    print(f"Ingesting document: {abs_path}")
    print(f"Document type: {document_type}")
    print(f"Using server: {server_binary}")

    # Send ingest request
    response = send_mcp_request("ingest", [abs_path, document_type], str(server_binary))

    if response is None:
        print("Failed to get response from server")
        return False

    if "error" in response:
        print(f"Server error: {response['error']}")
        return False

    if "result" in response:
        result = response["result"]
        print(f"✅ Success! Created {result.get('chunks_created', 0)} chunks")
        print(f"Document ID: {result.get('document_path', 'unknown')}")
        return True
    else:
        print("Unexpected response format")
        print(json.dumps(response, indent=2))
        return False

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    document_path = sys.argv[1]
    document_type = sys.argv[2] if len(sys.argv) > 2 else None

    success = ingest_document(document_path, document_type)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()