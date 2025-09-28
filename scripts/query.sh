#!/bin/bash
# Simple wrapper for the query script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$SCRIPT_DIR/query.py" "$@"