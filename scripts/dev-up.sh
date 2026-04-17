#!/usr/bin/env bash
# 一键起 dev 环境:docker-compose 起 pg → 等 ready → 跑 db-reset.sh

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

echo "=== 启动 postgres ==="
docker compose up -d pg

echo "=== 等 postgres ready ==="
for i in $(seq 1 30); do
  if docker compose exec -T pg pg_isready -U cuba -d cuba_ims > /dev/null 2>&1; then
    echo "postgres ready ✓"
    break
  fi
  sleep 1
done

if ! docker compose exec -T pg pg_isready -U cuba -d cuba_ims > /dev/null 2>&1; then
  echo "❌ postgres 超时未就绪"
  exit 1
fi

echo ""
echo "=== 跑 migrations + seed ==="
"$root/scripts/db-reset.sh" --keep-volume || "$root/scripts/db-reset.sh"

echo ""
echo "✅ dev 环境就绪"
echo "   DATABASE_URL=postgres://cuba:cuba@localhost:5432/cuba_ims"
echo ""
echo "启动服务: cargo run -p cuba-server"
