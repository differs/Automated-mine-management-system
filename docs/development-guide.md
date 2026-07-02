# 开发指南

## 技术栈

| 层 | 技术 | 版本 |
|----|------|------|
| 后端语言 | Rust | 1.85+ (edition 2024) |
| Web 框架 | axum | 0.8 |
| 数据库 ORM | sqlx | 0.8 |
| 异步运行时 | tokio | 1.48 |
| 缓存/队列 | Redis (redis-rs) | 0.27 |
| 实时通信 | WebSocket (axum) | 内置 |
| 前端框架 | Vite + Vue 3 | 最新 |
| 移动端 | Flutter / H5 | - |

## 项目结构

```
auto-mining-system/
├── apps/
│   ├── api/              # Rust 后端 API 服务
│   │   ├── src/
│   │   │   ├── main.rs       # 入口
│   │   │   ├── lib.rs        # 模块导出
│   │   │   ├── app.rs        # 路由构建
│   │   │   ├── config.rs     # 配置管理
│   │   │   ├── state.rs      # 应用状态
│   │   │   ├── error.rs      # 错误处理
│   │   │   ├── middleware/   # 中间件（Auth, Logger）
│   │   │   └── modules/
│   │   │       ├── auth/       # 认证模块
│   │   │       ├── driver/     # 司机模块
│   │   │       ├── pit/        # 坑口模块
│   │   │       ├── waybill/    # 运单模块
│   │   │       ├── queue/      # 排队模块
│   │   │       ├── loading/    # 装车模块
│   │   │       ├── weighing/   # 称重模块
│   │   │       ├── dashboard/  # 看板模块
│   │   │       ├── alert/      # 告警模块
│   │   │       └── ws/         # WebSocket 模块
│   │   ├── Cargo.toml
│   │   └── Dockerfile
│   ├── admin-web/         # 调度后台 (Vite + Vue 3)
│   │   ├── src/
│   │   │   ├── views/        # 页面
│   │   │   ├── components/   # 组件
│   │   │   ├── api/          # API 调用
│   │   │   ├── router/       # 路由
│   │   │   └── stores/       # 状态管理
│   │   └── Dockerfile
│   ├── driver-app/        # 司机端 (Flutter) - 规划中
│   ├── driver-miniapp/    # 司机端 (H5/小程序)
│   ├── pit-app/           # 坑口端 (Flutter) - 规划中
│   └── pit-h5/            # 坑口端 (H5)
├── db/
│   ├── init.sql              # 初始化建表
│   └── migrations/           # 迁移脚本
│       ├── 0001_mvp_init.sql
│       └── 0002_real_scene_extensions.sql
├── docs/                     # 文档
├── infra/                    # 部署配置
├── docker-compose.yml        # 开发环境
├── Cargo.toml                # 工作空间
└── package.json              # Node 工作空间
```

## 文档参考

项目文档位于 `docs/` 目录：

| 文档 | 说明 |
|------|------|
| `product-overview.md` | 产品概览 |
| `requirements-baseline.md` | 一期需求基线 |
| `architecture.md` | 系统架构设计 |
| `scenario-coverage-analysis.md` | 真实场景覆盖度分析 |
| `api-reference.md` | API 参考文档 |
| `deployment-guide.md` | 部署指南 |
| `database-schema.md` | 数据库 Schema 说明 |
| `user-manual.md` | 用户操作手册 |
| `phase-plan.md` | 分期实施计划 |
| `contact-info.md` | 投递联系信息 |
| `real-site-adaptation-analysis.md` | 实际矿山场景适配分析 |
| `requirement-coverage-analysis.md` | 招聘需求 vs 系统实现覆盖分析 |

### 需求覆盖状态

当前系统实现与目标招聘岗位需求的完整对比，详见 `docs/requirement-coverage-analysis.md`。

**核心结论：**
- 基础调度流程：**100%** 覆盖（创建→派单→到场→排队→装车→称重→完成）
- 调度员后台：**90%** 覆盖（登录/看板/运单/司机/坑口/队列/告警）
- 司机移动端：**70%** 覆盖（任务/签到/排队/取消，缺历史记录+实时推送）
- 坑口作业端：**70%** 覆盖（队列/叫号/装车/称重，缺独立地磅端）

**当前未完成的缺口（按优先级排序）：**
1. 司机端历史记录页面
2. 看板效率排名图表展示
3. 地磅独立操作端（H5）
4. 财务/运营专用视图
5. 批量导入真实逻辑（当前为 mock）
6. 司机端 WebSocket 替代轮询
7. 离线/弱网缓存能力

## 本地开发

### 后端开发（Rust）

#### 前提条件

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly

# 安装 cargo-watch（热重载）
cargo install cargo-watch

# 安装 sqlx-cli（迁移管理）
cargo install sqlx-cli --features postgres
```

#### 启动数据库

```bash
docker compose up -d postgres redis
```

#### 运行迁移

```bash
cargo run -p api --bin migrate
```

#### 开发模式运行

```bash
# 热重载开发
cargo watch -x "run -p api"
```

#### 运行测试

```bash
# 全部测试
docker compose run --rm api-test

# 仅单元测试
cargo test -p api --lib

# 仅集成测试
cargo test -p api --test '*'
```

#### 代码规范

```bash
# 格式化
cargo fmt -p api

# Lint
cargo clippy -p api -- -D warnings
```

### 前端开发（Admin Web）

#### 前提条件

```bash
# 安装 Node.js 20+
nvm install 20
```

#### 启动开发服务器

```bash
# 安装依赖
npm install

# 启动开发服务器（端口 5173，代理 API 到 3000）
npm run dev:admin
```

#### 构建

```bash
npm run build:admin
```

### 开发工作流

#### 1. 添加新 API 端点

```
steps:
  1. 在对应模块的 .rs 文件中添加路由处理函数
  2. 在 router() 中注册新路由
  3. 如果涉及新资源组，在 modules/mod.rs 中导出
  4. 在 app.rs 中挂载路由
  5. 如果需要新数据库表，在 db/migrations/ 创建迁移
  6. 更新 docs/api-reference.md
```

#### 2. 添加数据库迁移

```bash
# 手动创建迁移文件
touch db/migrations/0003_feature_name.sql

# 迁移文件格式
-- 0003_feature_name.sql
-- Description: 添加功能 XXX

-- 使用 IF NOT EXISTS 保证幂等性
-- 所有变更写在 DO $$ ... $$ 块中

-- 在 init.sql 中同步更新
```

#### 3. 提交规范

遵循 Conventional Commits：

```
feat(api): add alert notification endpoint
fix(queue): resolve race condition on concurrent join
docs: add deployment guide
refactor(modules): extract common validation logic
test(waybill): add integration tests for cancel flow
```

## Codeworkspace 配置

在项目根目录创建 `.codex` 文件：

```json
{
  "commands": {
    "dev": "docker compose up",
    "test": "docker compose run --rm api-test",
    "lint": "cargo clippy -p api -- -D warnings",
    "db:migrate": "cargo run -p api --bin migrate",
    "db:seed": "cargo run -p api --bin seed"
  }
}
```

## 调试技巧

### Rust 后端

```bash
# 开启详细日志
RUST_LOG=api=trace cargo run -p api

# 性能分析
cargo build --release
perf record ./target/release/api
perf report

# 内存检查
cargo install cargo-valgrind
cargo valgrind -p api
```

### PostgreSQL

```bash
# 查看慢查询
docker exec -it auto-mining-postgres psql -U postgres auto_mining_system -c "
  SELECT query, calls, total_time / calls AS avg_time
  FROM pg_stat_statements
  ORDER BY total_time DESC LIMIT 10;"

# 查看锁
docker exec -it auto-mining-postgres psql -U postgres auto_mining_system -c "
  SELECT pid, locktype, mode, granted
  FROM pg_locks WHERE NOT granted;"
```

### Redis

```bash
# 查看队列状态
docker exec -it auto-mining-redis redis-cli KEYS "pit:*:queue"

# 监控实时操作
docker exec -it auto-mining-redis redis-cli MONITOR
```

## 常见问题

### Q: cargo build 很慢怎么办？

启用增量编译和缓存：

```bash
# Cargo.toml
[profile.dev]
codegen-units = 256
incremental = true

# 设置 sccache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

### Q: docker compose 中 PostgreSQL 端口冲突？

```yaml
# 修改 docker-compose.yml 映射端口
ports:
  - "5433:5432"  # 宿主用 5433 连接
```

### Q: WebSocket 连接不上？

1. 确认 token 有效
2. 检查防火墙是否放通端口
3. 检查 Nginx 是否正确配置了 WebSocket 升级头
