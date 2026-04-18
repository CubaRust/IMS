# syntax=docker/dockerfile:1.7
# ------------------------------------------------------------
# Multi-stage build for cuba-server
#
# Stage 1: chef-plan  —— 仅解析依赖,提取 recipe.json
# Stage 2: builder    —— 用 recipe 预编译依赖,再编译业务代码
# Stage 3: runtime    —— 极小 distroless 镜像,仅带二进制和 migrations
# ------------------------------------------------------------

ARG RUST_VERSION=1.82
ARG DEBIAN_VERSION=bookworm

# =============================================================================
# chef-plan: 准备依赖 recipe
# =============================================================================
FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS chef
RUN cargo install --locked cargo-chef
WORKDIR /app

FROM chef AS chef-plan
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY migrations ./migrations
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# builder: 编译依赖 + 业务
# =============================================================================
FROM chef AS builder
ENV CARGO_TERM_COLOR=always

# 装 sqlx 可能依赖的系统库(rustls 避免 openssl 麻烦)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=chef-plan /app/recipe.json recipe.json

# 依赖预构建(此层被 recipe 驱动,代码变更不会使其失效)
RUN cargo chef cook --release --recipe-path recipe.json --bin cuba-server

# 拷真实源码再构建业务
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY migrations ./migrations

ENV SQLX_OFFLINE=true
RUN cargo build --release --bin cuba-server

# =============================================================================
# runtime: 只带 binary + migrations(~50 MB)
# =============================================================================
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/cuba-server /usr/local/bin/cuba-server
COPY --chown=nonroot:nonroot migrations /app/migrations

ENV APP_ENV=prod \
    HTTP_HOST=0.0.0.0 \
    HTTP_PORT=8080 \
    MIGRATION_MODE=manual \
    RUST_LOG=info,cuba_=info

EXPOSE 8080

USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/cuba-server"]
