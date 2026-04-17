#!/usr/bin/env bash
# 运行本项目全部测试(单测 + e2e)
# 依赖:docker(testcontainers 使用)、cargo

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

export RUST_LOG="${RUST_LOG:-warn,cuba_=info}"
export CUBA_TEST_LOG="${CUBA_TEST_LOG:-warn}"

echo "=== fmt check ==="
cargo fmt --all -- --check

echo ""
echo "=== clippy ==="
cargo clippy --workspace --all-targets --tests -- -D warnings

echo ""
echo "=== build ==="
cargo build --workspace --all-targets

echo ""
echo "=== unit tests ==="
cargo test --workspace --exclude cuba-e2e --lib

echo ""
echo "=== e2e tests ==="
if ! docker version > /dev/null 2>&1; then
  echo "docker 未就绪,跳过 e2e。请启动 Docker 后再跑 e2e。"
  exit 0
fi
cargo test -p cuba-e2e -- --test-threads=1

echo ""
echo "✅ 全部测试通过"
