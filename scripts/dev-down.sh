#!/usr/bin/env bash
# 停 dev 环境

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ "${1:-}" == "--purge" ]]; then
  echo "=== 停 + 删 volume ==="
  docker compose down -v
else
  echo "=== 停(保留 volume,下次 dev-up 数据还在) ==="
  docker compose down
fi

echo "✅ 已停止"
