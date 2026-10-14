#!/usr/bin/env bash
# S1: build per-bank IDA databases and export mechanical JSON.
# 薄包装：导出逻辑只存在于 tools/ida/run_all_banks.py（跨平台，无第二套逻辑）。
set -euo pipefail
exec python3 "$(dirname "$0")/run_all_banks.py" "$@"
