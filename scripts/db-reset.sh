#!/usr/bin/env bash
# =============================================================================
# scripts/db-reset.sh
# 一键重置开发数据库
#
# 用法:
#   ./scripts/db-reset.sh                  # drop + create + migrate + verify
#   ./scripts/db-reset.sh --with-demo      # 额外执行 demo_data.sql
#   ./scripts/db-reset.sh --no-verify      # 跳过 verify.sql
#   ./scripts/db-reset.sh --no-drop        # 不 drop,仅增量 migrate(适合已有数据)
#
# 前置:
#   1. cargo install sqlx-cli --no-default-features --features rustls,postgres
#   2. 环境变量 DATABASE_URL 已设置(或项目根有 .env)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

WITH_DEMO=0
DO_VERIFY=1
DO_DROP=1

for arg in "$@"; do
    case "$arg" in
        --with-demo) WITH_DEMO=1 ;;
        --no-verify) DO_VERIFY=0 ;;
        --no-drop)   DO_DROP=0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

# 加载 .env
if [ -f "$PROJECT_ROOT/.env" ]; then
    set -a
    # shellcheck disable=SC1091
    source "$PROJECT_ROOT/.env"
    set +a
fi

if [ -z "${DATABASE_URL:-}" ]; then
    echo "❌ DATABASE_URL not set. Export it or put it in .env"
    exit 1
fi

# 依赖检查
command -v sqlx >/dev/null 2>&1 || {
    echo "❌ sqlx-cli 未安装"
    echo "   cargo install sqlx-cli --no-default-features --features rustls,postgres"
    exit 1
}
command -v psql >/dev/null 2>&1 || {
    echo "❌ psql 未安装(需要 PostgreSQL 客户端)"
    exit 1
}

echo "📦 DATABASE_URL = $DATABASE_URL"

if [ "$DO_DROP" -eq 1 ]; then
    echo "🗑️  Drop database..."
    sqlx database drop -y || true
    echo "📁 Create database..."
    sqlx database create
fi

echo "🔧 Run migrations..."
sqlx migrate run --source "$PROJECT_ROOT/migrations"

if [ "$WITH_DEMO" -eq 1 ]; then
    echo "🎭 Load demo data..."
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$PROJECT_ROOT/scripts/seed-data/demo_data.sql"
fi

if [ "$DO_VERIFY" -eq 1 ]; then
    echo "🔍 Verify..."
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$PROJECT_ROOT/scripts/verify.sql"
fi

echo ""
echo "✅ Database ready."
echo "   - admin / Admin@123  (上线前必须改)"
