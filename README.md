# EQYCC_CUBA_IMS

手机屏幕组装厂的仓储管理系统(WMS)后端。Rust + Axum + sqlx + PostgreSQL。

**状态**:M1 功能完整 ✅ · M2 测试 & DX ✅ · M3 生产运行时 ✅(metrics / 容器化 / Helm / JWT 吊销 / 健康检查)。

---

## 3 分钟上手(新同事必读)

```bash
# 1. 装依赖
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust
# Docker Desktop 或 docker-ce
cargo install just                                                # 可选但强烈推荐

# 2. 起库 + 启服务
just dev        # docker 起 pg + 跑 migrations + seed
just run        # 启动 API

# 3. 打第一个请求
curl -sX POST http://localhost:8080/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"login_name":"admin","password":"Admin@123"}' | jq
```

浏览器打开 `http://localhost:8080/docs` 看 Swagger UI。

---

## 文档入口

| 文档 | 读者 | 内容 |
|---|---|---|
| [README-M1.md](./README-M1.md) | PM / 架构师 / 业务分析 | 领域建模、crate 总览、业务流程、端到端示例 |
| [README-M2.md](./README-M2.md) | 后端工程师 | 开发循环、加测试/加端点、CI、FAQ |
| [CHANGELOG.md](./CHANGELOG.md) | 所有人 | 版本变更 |
| `docs/` | 深度读者 | 架构决策 / DDD 边界 / 错误码 |

---

## 一页总览

```
15 业务 crate + 5 基础设施 crate + 2 测试 crate
16 个 SQL migration(0001-0016,0011 独立作为 demo)
~90 个 HTTP 端点 / 14 个 e2e 测试文件
5 类库存事务(IN/OUT/TRANSFER/CONVERT/RESERVE+RELEASE)单一引擎
异常先发允许 PREISSUE_PENDING 负库存,自动冲销闭环
审计日志异步落 sys_audit_log;trace_id 全链路贯通
Prometheus /metrics:HTTP QPS/延迟、库存 txn 成败、业务错误码分布
Swagger UI 挂在 /docs;OpenAPI JSON 在 /api-docs/openapi.json
4 个 CI job:fmt+clippy / unit / e2e(testcontainers) / migrations
```

---

## 常用命令

```bash
just             # 列出所有命令
just dev         # 起 dev 环境
just run         # 启动 API
just test        # 全流程测试
just unit        # 只单测(不用 docker)
just e2e         # 只 e2e(需要 docker)
just e2e-one preissue_cycle   # 单个 e2e
just fmt-check   # 格式检查
just clippy      # 静态检查
just psql        # 连 dev pg
just balance     # 看余额(快速调试)
just flow        # 看最近流水
just dev-purge   # 清库重置
```

不用 just 的同事:`scripts/test.sh`、`scripts/dev-up.sh` 等价。

---

## 核心约定(违反即拒)

1. **所有库存变化必须过 `InventoryService::commit`**;业务 crate 不允许直接写 `inv.balance`
2. **业务 crate 不依赖 axum**;HTTP 层一律在 `cuba-api`
3. **业务错误 HTTP 200 + `code` 字段**;只有 401/403/500 用 HTTP status
4. **单据状态机**:DRAFT → SUBMITTED → COMPLETED / VOIDED(void 仅限前两者)
5. **头+行创建走单 DB 事务**(pool.begin 到 commit 之间不能跨 HTTP)
6. **错误码分段**:10xxx 共享 / 11xxx identity / 20xxx inventory / 21xxx warehouse /
   22xxx catalog / 30xxx inbound / 31xxx outbound / 33xxx preissue / 40xxx defect /
   41xxx recovery / 42xxx scrap / 44xxx customer-return / 45xxx supplier-return /
   46xxx pmc / 47xxx stocktake

---

## 许可

Proprietary(EQYCC 内部)
