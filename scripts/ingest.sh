#!/bin/bash
# Simple wrapper for the ingest script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$SCRIPT_DIR/ingest.py" "$@"