# 部署指南

## 环境要求

| 组件 | 版本要求 | 说明 |
|------|---------|------|
| Docker | 24.0+ | 容器运行环境 |
| Docker Compose | 2.20+ | 多容器编排 |
| PostgreSQL | 16+ | 主数据库 |
| Redis | 7+ | 缓存与实时队列 |

## 开发环境部署

### 1. 克隆项目

```bash
git clone <repo-url> auto-mining-system
cd auto-mining-system
```

### 2. 配置环境变量

```bash
cp .env.example .env
```

编辑 `.env` 文件：

```env
APP_HOST=127.0.0.1
APP_PORT=3000
DATABASE_URL=postgres://postgres:postgres@localhost:5432/auto_mining_system
RUST_LOG=api=debug,tower_http=info
JWT_SECRET=your-secret-key-here
REDIS_URL=redis://localhost:6379
```

### 3. 一键启动

```bash
docker compose up --build
```

这会依次启动：
1. PostgreSQL 数据库（端口 5432，仅在 compose 内网）
2. Redis 缓存（端口 6379，仅在 compose 内网）
3. 数据库迁移（自动执行）
4. API 服务（暴露端口 3000）

### 4. 验证部署

```bash
# 健康检查
curl http://localhost:3000/health

# 预期返回
{"service":"automated-mine-management-api","status":"ok"}
```

### 5. 初始化种子数据

```bash
# 运行种子数据脚本
docker compose run --rm api /usr/local/bin/api seed
```

## 生产环境部署

### 架构拓扑

```
                    ┌─────────────┐
                    │   Nginx     │  <-- 反向代理 / SSL 终止
                    │  (LB)       │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼─────┐ ┌──▼──┐  ┌───▼────┐
        │  API Svr  │ │ API │  │  API   │
        │  (主)      │ │ Svr │  │  Svr   │
        └─────┬─────┘ └──┬──┘  └───┬────┘
              │            │        │
              └────────────┼────────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼─────┐ ┌──▼──┐  ┌───▼────┐
        │PostgreSQL │ │Redis│  │  NATS  │
        │ (主从)     │ │    │  │ (可选  │
        └───────────┘ └─────┘  │ 通知)   │
                               └────────┘
```

### 1. 构建生产镜像

```bash
# 构建 API
docker build -t auto-mining-api:latest -f apps/api/Dockerfile .

# 构建 Admin Web
docker build -t auto-mining-admin:latest -f apps/admin-web/Dockerfile .
```

### 2. 生产配置

```yaml
# docker-compose.prod.yml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_DB: auto_mining_system
      POSTGRES_USER: ${DB_USER}
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - /data/postgres:/var/lib/postgresql/data
    restart: always
    deploy:
      resources:
        limits:
          memory: 4G

  redis:
    image: redis:7-alpine
    volumes:
      - /data/redis:/data
    restart: always

  api:
    image: auto-mining-api:latest
    depends_on: [postgres, redis]
    environment:
      DATABASE_URL: postgres://${DB_USER}:${DB_PASSWORD}@postgres:5432/auto_mining_system
      REDIS_URL: redis://redis:6379
      JWT_SECRET: ${JWT_SECRET}
      RUST_LOG: info
    restart: always
    deploy:
      replicas: 2
      resources:
        limits:
          memory: 512M

  admin-web:
    image: auto-mining-admin:latest
    ports:
      - "8080:80"
    restart: always

  nginx:
    image: nginx:alpine
    volumes:
      - ./infra/nginx.conf:/etc/nginx/nginx.conf
      - /etc/ssl/certs:/etc/ssl/certs:ro
    ports:
      - "443:443"
      - "80:80"
    depends_on: [api, admin-web]
    restart: always
```

### 3. Nginx 配置

```nginx
# infra/nginx.conf
upstream api_backend {
    server api:3000;
}

server {
    listen 80;
    server_name mining.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl;
    server_name mining.example.com;

    ssl_certificate /etc/ssl/certs/example.crt;
    ssl_certificate_key /etc/ssl/certs/example.key;

    # Admin Web 静态文件
    location / {
        proxy_pass http://admin-web:80;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # API 代理
    location /api/ {
        proxy_pass http://api_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # WebSocket 代理
    location /ws {
        proxy_pass http://api_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
    }
}
```

### 4. 启动生产环境

```bash
docker compose -f docker-compose.prod.yml up -d
```

## 数据库管理

### 备份

```bash
# 全量备份
docker exec -t auto-mining-postgres pg_dump -U postgres auto_mining_system > backup_$(date +%Y%m%d).sql

# 定时备份（crontab）
0 2 * * * docker exec -t auto-mining-postgres pg_dump -U postgres auto_mining_system > /backups/db_$(date +\%Y\%m\%d).sql
```

### 恢复

```bash
cat backup_20260101.sql | docker exec -i auto-mining-postgres psql -U postgres auto_mining_system
```

### 迁移

```bash
# 手动执行迁移
docker compose run --rm api migrate

# 查看迁移状态
docker compose run --rm api migrate --dry-run
```

## 监控与运维

### 健康检查端点

| 端点 | 用途 |
|------|------|
| `/health` | 基础健康检查 |
| `/health/db` | 数据库连接检查 |
| `/health/redis` | Redis 连接检查 |

### 日志查看

```bash
# API 日志
docker compose logs -f api

# 数据库日志
docker compose logs -f postgres

# 按时间过滤
docker compose logs --since="2026-01-01T10:00:00" api
```

### 性能监控

推荐使用以下工具：
- **Prometheus** + **Grafana**：收集和展示 API 指标
- **pg_stat_statements**：PostgreSQL 慢查询分析
- **RedisInsight**：Redis 监控

## 扩容指南

### 水平扩容

```bash
# 增加 API 实例数
docker compose -f docker-compose.prod.yml up -d --scale api=5
```

### 数据库只读副本

```yaml
# 扩展 docker-compose.prod.yml
postgres-replica:
  image: postgres:16
  environment:
    POSTGRES_PRIMARY_URL: postgres://${DB_USER}:${DB_PASSWORD}@postgres:5432
  volumes:
    - /data/postgres-replica:/var/lib/postgresql/data
```

## 故障恢复

### API 服务故障

```bash
# 重启 API
docker compose restart api

# 滚动更新
docker compose up -d --no-deps --scale api=2 api
```

### 数据库故障

```bash
# 从备份恢复
cat backup_latest.sql | docker exec -i auto-mining-postgres psql -U postgres auto_mining_system

# 重建容器
docker compose down postgres
docker compose up -d postgres
docker compose run --rm api migrate
```

## 安全建议

1. **JWT Secret**：使用强随机密钥，长度不低于 32 字符
2. **数据库密码**：生产环境使用独立密码，定期更换
3. **网络隔离**：PostgreSQL 和 Redis 不对外暴露端口
4. **HTTPS**：强制 HTTPS，SSL 证书使用 Let's Encrypt
5. **定期备份**：数据库每日自动备份，保留 30 天
6. **审计日志**：所有关键操作记录到 `operation_logs` 表
7. **权限最小化**：按角色分配最小必要权限
