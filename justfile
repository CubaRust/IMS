# EQYCC CUBA IMS — 开发快捷命令
# 安装 just:  cargo install just
# 用法:     just <command>
#
# 常用:
#   just           显示命令列表
#   just dev       起 pg + migrations
#   just run       启动服务
#   just test      全测试
#   just e2e       只跑 e2e
#   just docs      启动服务并打开 swagger

# 默认显示帮助
default:
    @just --list

# ---------- 环境 ----------

# 起 docker-compose postgres + 跑 migrations + seed
dev:
    ./scripts/dev-up.sh

# 停 docker-compose(保留数据)
dev-down:
    ./scripts/dev-down.sh

# 停 + 清 volume(彻底重置)
dev-purge:
    ./scripts/dev-down.sh --purge

# 重置 DB(不动 container,只 drop/create/migrate)
db-reset:
    ./scripts/db-reset.sh

# ---------- 构建 ----------

# 增量构建
build:
    cargo build --workspace

# release 构建
release:
    cargo build --workspace --release

# 清理
clean:
    cargo clean

# ---------- 运行 ----------

# 启动 API 服务(默认 dev 环境变量)
run:
    cargo run -p cuba-server

# release 模式启动
run-release:
    cargo run -p cuba-server --release

# 启动服务 + 在浏览器打开 swagger(需要 xdg-open 或 macOS open)
docs:
    @echo "启动服务后访问 http://localhost:8080/docs"
    cargo run -p cuba-server &
    sleep 3 && (xdg-open http://localhost:8080/docs 2>/dev/null \
        || open http://localhost:8080/docs 2>/dev/null \
        || echo "请手动打开 http://localhost:8080/docs")

# ---------- 测试 ----------

# 全部测试(fmt + clippy + build + unit + e2e)
test:
    ./scripts/test.sh

# 只跑单测(不起 docker)
unit:
    cargo test --workspace --exclude cuba-e2e --lib

# 只跑 e2e(需要 docker 运行中)
e2e:
    cargo test -p cuba-e2e -- --test-threads=1

# 跑指定 e2e 文件(示例:just e2e-one inbound_outbound)
e2e-one name:
    cargo test -p cuba-e2e --test e2e_{{name}} -- --test-threads=1 --nocapture

# ---------- 质量 ----------

# 格式化
fmt:
    cargo fmt --all

# 格式检查
fmt-check:
    cargo fmt --all -- --check

# Clippy
clippy:
    cargo clippy --workspace --all-targets --tests -- -D warnings

# 修 clippy 建议(谨慎,会改代码)
clippy-fix:
    cargo clippy --workspace --all-targets --tests --fix --allow-dirty -- -D warnings

# ---------- 数据库工具 ----------

# 连上 dev pg(需要 psql)
psql:
    docker compose exec pg psql -U cuba -d cuba_ims

# 看 balance 表
balance:
    docker compose exec pg psql -U cuba -d cuba_ims -c "select * from inv.balance order by updated_at desc limit 20;"

# 看最近 20 条事务流水
flow:
    docker compose exec pg psql -U cuba -d cuba_ims -c "select id, txn_no, txn_type, scene_code, doc_type, doc_no, created_at from inv.txn_h order by id desc limit 20;"
