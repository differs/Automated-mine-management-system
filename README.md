# 矿山自动化调度系统

[English Version](./README.en.md)

面向矿山运输场景的数字化调度产品。把矿山现场最核心的一条业务链路数字化：从调度派车，到司机到场排队，再到装车、称重、完单和数据沉淀，逐步替代微信群、电话和 Excel。

## 产品定位

解决矿山运输调度三类核心问题：

- **调度效率低**：派单靠电话和微信群，信息不同步
- **现场秩序差**：坑口排队不可视，插队和扯皮频发
- **经营数据滞后**：趟次、吨位、异常、效率都要靠人工汇总

一期目标：

- 能跑：派单、到场、排队、装车、称重、完单形成闭环
- 能控：关键节点留痕，人工干预可追溯，操作日志审计
- 能看：调度室实时看板能看到车、坑、单、队列和异常

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 后端 | Rust (axum 0.8 + sqlx 0.8) | 高性能异步 API 服务 |
| 缓存 | Redis 7 | 队列计数缓存、看板缓存、限流 |
| 数据库 | PostgreSQL 16 | 业务主库，20+ 张表 |
| 前端 | Vue 3 + Vite 6 + TypeScript | 调度后台 Web 管理端 |
| 移动端 | Flutter | 司机端 + 坑口端 App |
| 部署 | Docker Compose | 一键启动全部服务 |
| 认证 | JWT (HS256) | access token 24h + refresh token 30d |
| 实时通信 | WebSocket | 7 种事件类型实时推送 |
| 调度算法 | WPMA + AI 增强 | 纯算法/盘古大模型双模式，运行时切换 |

## 功能模块

### 后端 API（19 个模块）

| 模块 | 功能 | 亮点 |
|------|------|------|
| **auth** | 登录/刷新令牌 | bcrypt 验证，双 token 模式 |
| **driver** | 司机 CRUD + 搜索 + 批量导入 | 关键字搜索，唯一约束 |
| **pit** | 坑口 CRUD | 实时队列计数 |
| **waybill** | 运单全生命周期 | 9 状态严格校验，4 种到场方式 |
| **queue** | 排队/叫号/离队 | 事务+行锁，Redis 写穿 |
| **loading** | 装车开始/完成 | 事务控制，记录联动 |
| **weighing** | 称重完单 | 非负校验，自动完成 |
| **dashboard** | 运营看板 | LATERAL JOIN 高性能查询，Redis 缓存 30s |
| **dispatch** | 智能调度推荐 | WPMA 算法 + AI 增强，top-N 推荐 |
| **ai** | 调度算法引擎 | WPMA 加权优先级匹配 + 盘古大模型 |
| **ws** | WebSocket 推送 | 7 种事件，JWT 认证 |
| **alert** | 告警管理 | QueryBuilder 安全查询 |
| **fence** | 电子围栏 | Haversine 距离，自动到场 |
| **scale** | 地磅采集 | 串口/蓝牙，防作弊校验 |
| **missions** | 无人矿卡任务 | claim/status/complete 流程 |
| **offline** | 离线同步 | 幂等键+乐观锁，事务化 |
| **system_config** | 运行时配置 | 纯算法/AI 模式切换（Arc<RwLock>） |
| **audit_log** | 操作日志 | fire-and-forget 审计 |
| **health** | 健康检查 + OpenAPI | `GET /docs/openapi.json` |

### 运单状态机

```
pending_dispatch → dispatched → arrived → queueing → loading → loaded → weighing → completed
                  ↓
               cancelled (任何非终态均可取消，需填原因)
```

**四种到场方式**：手动签到、车牌扫描、电子围栏、离线同步

### 中间件

- **JWT 认证**：受保护路由自动验证 Bearer token
- **Redis 限流**：滑动窗口 100 次/60 秒/IP
- **CORS**：跨域支持
- **请求追踪**：TraceLayer

### 前端应用

- **admin-web**：调度后台（8 个页面 + 4 个通用组件）
- **driver-miniapp**：司机端 H5（轻量版）
- **pit-h5**：坑口端 H5（轻量版）
- **demo-hub**：演示中心门户

### Flutter 应用

- **driver-app**：司机端 App（任务接收、到场打卡、车牌识别、离线同步）
- **pit-app**：坑口端 App（车辆核验、排队管理、装车确认）
- **shared**：共享库（离线调度引擎 + 车牌 OCR + 地理围栏）

### 数据库

- 5 次迁移，20+ 张表
- PostgreSQL 枚举类型（6 种状态枚举）
- 乐观锁（waybills.version）
- 条件唯一索引（单司机单活跃运单）
- Check 约束（重量非负、取消必须有原因）

## 项目结构

```text
apps/
  api/              Rust API 服务（19 个业务模块）
  admin-web/        Vue 3 调度后台
  driver-app/       Flutter 司机端
  driver-miniapp/   司机端 H5
  pit-app/          Flutter 坑口端
  pit-h5/           坑口端 H5
  demo-hub/         演示中心门户
  shared/           Flutter 共享库（离线+车牌+围栏）
db/
  init.sql          数据库初始化脚本
  migrations/       5 个迁移文件
docs/               40+ 技术文档 + OpenAPI 规范
scripts/            爬虫和邮件脚本
```

## 本地运行

### 启动后端

```bash
docker compose up --build
```

PostgreSQL 和 Redis 在 compose 内网运行。API 暴露在本机 `3000` 端口。

### 启动前端

```bash
# 安装依赖（首次）
npm install

# 调度后台（端口 5173）
npm run dev:admin

# 司机端（端口 5174）
npm run dev:driver

# 坑口端（端口 5175）
npm run dev:pit

# 演示中心门户（端口 5180）
npm run dev:demo
```

所有前端开发服务器会自动代理 `/api` 请求到 `localhost:3000`。

### 构建前端

```bash
npm run build:all
# 或单独构建
npm run build:admin
npm run build:driver
npm run build:pit
npm run build:demo
```

### 运行测试

```bash
# 容器内测试（推荐）
docker compose run --rm api-test

# 宿主机测试
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p api
```

### 执行迁移

```bash
cargo run -p api --bin migrate
```

## API 文档

- OpenAPI 规范：`GET /docs/openapi.json`
- 完整 API 参考：`docs/api-reference.md`
- 部署指南：`docs/deployment-guide.md`

## 算法说明

调度算法基于 **通用框架 + 地形适配** 设计：

- **WPMA 算法**：加权优先级匹配（空闲权重 0.3 + 工作量 0.2 + 距离 0.3 + 坑口优先级 0.2）
- **AI 增强模式**：调用盘古大模型优化决策
- **双模式切换**：`POST /api/v1/system/dispatch-mode` 运行时切换，无需重启
- **拥堵预测**：阈值判断（>10 高，>5 中，其他低）
- **异常检测**：超时规则（>30 分钟异常，>15 分钟注意）

详见 `docs/dispatch-algorithm.md`

## 文档目录

| 文档 | 说明 |
|------|------|
| `docs/architecture.md` | 系统架构 |
| `docs/api-reference.md` | API 参考文档 |
| `docs/deployment-guide.md` | 部署指南 |
| `docs/database-schema.md` | 数据库 Schema |
| `docs/dispatch-algorithm.md` | 调度算法详解 |
| `docs/development-guide.md` | 开发指南 |
| `docs/user-manual.md` | 用户操作手册 |
| `docs/phase-plan.md` | 分期实施计划 |
| `docs/openapi.json` | OpenAPI 3.0 规范 |

## 用户角色

- **调度员**：看全局、派任务、处理异常、查看智能推荐
- **司机**：收任务、到场打卡、看排队、看历史
- **坑口管理员**：核验车辆、管理排队、确认装车
- **地磅操作员**：录入重量、完成称重
- **财务/运营**：查看产能、趟次、吨位和结算基础数据

## 测试覆盖

- 22 个单元测试（config、error、pagination、auth、ai 模块）
- 2 个集成测试（完整运单流程、状态校验）
- 全部通过

## 后续方向

- 报表导出（CSV/Excel）
- 多租户隔离
- 财务结算模块
- 消息通知（短信/微信）
- App 上架（iOS/Android）
- PostGIS 空间索引优化
