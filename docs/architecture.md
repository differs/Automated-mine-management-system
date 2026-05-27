# 系统架构设计

## 1. 设计原则

- 先跑通主流程，再扩展智能化能力
- 保证现场弱网可用，关键操作支持重试和幂等
- 所有状态变更可追溯
- 一期优先简单可靠，不引入过度复杂的分布式组件

## 2. 系统组成

```text
admin-web      调度后台（PC）
driver-miniapp 司机端（微信小程序）
pit-h5         坑口端（PAD/H5）
api            NestJS 业务服务
postgres       业务数据
redis          队列、缓存、短期状态
ws-gateway     实时推送
```

## 3. 逻辑分层

### 3.1 前端层

- 后台 Web：调度、管理、统计
- 小程序：司机任务、排队、历史记录
- 坑口端 H5：扫码核验、叫号、装车、称重提交

### 3.2 应用层

- Auth 模块：账号、司机身份、角色鉴权
- Driver 模块：司机和车辆档案
- Pit 模块：坑口配置和可用状态
- Waybill 模块：运单生命周期管理
- Queue 模块：到场、入队、叫号、离队
- Loading 模块：装车过程记录
- Weighing 模块：称重和异常校验
- Dashboard 模块：统计查询
- Alert 模块：规则告警

### 3.3 数据层

- PostgreSQL：主数据、运单、日志、报表基础
- Redis：
  - `pit:{pitId}:queue` 存放活动队列
  - `driver:{driverId}:active_waybill` 存放司机进行中任务
  - `dashboard:*` 存放看板短期缓存

## 4. 关键技术决策

### 4.1 后端

- 使用 NestJS 作为主服务框架
- 使用 Prisma 或 TypeORM 管理 ORM 和迁移
- 使用 WebSocket 推送调度变化和叫号消息
- 所有写操作引入幂等控制和状态校验

### 4.2 实时队列

- Redis 作为排队事实缓存
- PostgreSQL 持久化排队日志
- 进入队列时先写数据库事务，再刷新 Redis
- Redis 丢失时可从数据库活动状态重建

### 4.3 弱网处理

- 小程序和坑口端缓存当前任务和最近一次操作
- 写接口采用客户端请求 ID，服务端去重
- 对关键动作提供“提交中 / 已提交待同步 / 已成功”三态提示

## 5. 状态流转边界

### 5.1 允许发起状态变更的角色

| 状态变更 | 发起角色 |
|---|---|
| 待派车 -> 已派车 | 调度员 |
| 已派车 -> 已到场 | 司机 / 坑口管理员 |
| 已到场 -> 排队中 | 系统 / 坑口管理员 |
| 排队中 -> 装载中 | 坑口管理员 |
| 装载中 -> 已装载 | 坑口管理员 |
| 已装载 -> 称重中 | 地磅操作员 / 系统 |
| 称重中 -> 已完成 | 地磅操作员 / 系统 |
| 任意进行中 -> 已取消 | 调度员 / 管理员 |

### 5.2 并发控制

- 同一运单状态变更使用乐观锁版本号
- 同一司机派单前校验不存在进行中运单
- 同一坑口叫号前校验当前叫号目标仍在队列中

## 6. 模块间事件

一期可先在单体服务内通过领域事件实现。

| 事件 | 生产者 | 消费者 |
|---|---|---|
| `waybill.dispatched` | Waybill | Notify, Dashboard |
| `queue.joined` | Queue | Dashboard, Alert |
| `loading.started` | Loading | Dashboard |
| `weighing.completed` | Weighing | Waybill, Dashboard, Alert |
| `waybill.cancelled` | Waybill | Queue, Dashboard |

## 7. 权限模型

### 后台角色

- `super_admin`
- `dispatcher`
- `pit_operator`
- `weigh_operator`
- `finance`
- `ops_analyst`

### 司机端身份

- `driver`

## 8. 部署建议

### 一期最小部署

- 1 台应用服务器：Nginx + API + WebSocket
- 1 台数据库服务器：PostgreSQL + Redis
- 对象存储预留给图片和附件

### 网络建议

- 公有云优先，矿区使用 4G/5G 接入
- 如果矿区必须本地部署，建议做云端主库 + 现场边缘缓存的后续方案

## 9. 首批 API 范围

- `POST /auth/login`
- `GET /drivers`
- `POST /drivers/import`
- `GET /pits`
- `POST /waybills`
- `POST /waybills/:id/dispatch`
- `POST /waybills/:id/arrive`
- `POST /waybills/:id/queue/join`
- `POST /waybills/:id/loading/start`
- `POST /waybills/:id/loading/finish`
- `POST /waybills/:id/weigh`
- `POST /waybills/:id/cancel`
- `GET /dashboard/overview`

## 10. 建议 Monorepo 结构

```text
apps/
  admin-web/
  driver-miniapp/
  pit-h5/
  api/
packages/
  shared-types/
  shared-utils/
db/
docs/
infra/
```
