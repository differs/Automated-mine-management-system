# Auto Mining Dispatch System

矿山运输车辆调度系统项目基线仓库。

当前仓库用于沉淀一期 MVP 的业务基线、技术架构、数据库设计和实施路线，后续会在此基础上逐步补齐后台、司机端、小程序端、坑口端和部署脚本。

## 当前范围

- `docs/requirements-baseline.md`：一期需求基线
- `docs/architecture.md`：系统架构和模块设计
- `db/init.sql`：PostgreSQL 初版建表脚本

## 目标

替换“微信群 + 电话 + Excel”的矿山运输调度流程，跑通以下闭环：

派单 -> 到场 -> 排队 -> 装车 -> 称重 -> 完成 -> 结算

## 建议技术栈

- 管理后台：Vue 3 + Element Plus + Vite
- 司机端：微信小程序
- 坑口端：H5 / PAD
- 后端：NestJS + TypeScript
- 数据库：PostgreSQL + Redis
- 部署：Docker + Nginx

## 下一步

1. 明确一期字段和现场流程细节
2. 初始化 Git 仓库和 monorepo 工程
3. 落地 NestJS API、前端后台和小程序骨架
4. 建立测试、部署和环境配置规范
