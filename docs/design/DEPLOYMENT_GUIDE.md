# Octo Workbench 部署指南

> 本文档涵盖从本地开发到生产部署的完整流程。

---

## 1. 环境要求

| 依赖 | 最低版本 | 说明 |
|------|---------|------|
| Rust | 1.75+ | 后端编译 |
| Node.js | 18+ | 前端构建 |
| pnpm | 8+ | 前端包管理器（或 npm） |
| SQLite | 3.35+ | 内嵌数据库（通过 rusqlite 自动链接） |
| Docker | 20.10+ | 可选，容器化部署 |

---

## 2. 快速开始

### 2.1 克隆与构建

```bash
git clone https://github.com/uukuguy/octo-sandbox.git
cd octo-sandbox

# 安装前端依赖
make setup

# 创建环境配置
cp .env.example .env
# 编辑 .env，填入 ANTHROPIC_API_KEY

# 构建
make build       # Rust debug build
make web-build   # Frontend production build
```

### 2.2 启动开发模式

```bash
make dev          # 同时启动后端(3001) + 前端(5180)
```

或分别启动:

```bash
make server       # 仅后端 (端口 3001)
make web          # 仅前端 (端口 5180)
```

### 2.3 验证

```bash
curl http://localhost:3001/api/health
# 预期: {"status":"ok"}
```

---

## 3. 配置说明

### 3.1 配置优先级

```
.env (最高) > CLI 参数 > config.yaml (最低)
```

### 3.2 config.yaml 全字段参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| **server.host** | string | `127.0.0.1` | 服务器绑定地址 |
| **server.port** | u16 | `3001` | 服务器端口 |
| **server.cors_origins** | string[] | `[]` | 允许的 CORS 源（空=允许全部） |
| **provider.name** | string | `anthropic` | LLM 提供商：`anthropic` / `openai` |
| **provider.api_key** | string | - | API 密钥（推荐通过 .env 设置） |
| **provider.base_url** | string? | `null` | 自定义 API 端点（可选代理 URL） |
| **provider.model** | string? | `null` | 模型覆盖（如 `claude-sonnet-4-20250514`） |
| **database.path** | string | `./data/octo.db` | SQLite 数据库路径 |
| **logging.level** | string | `octo_server=debug,...` | RUST_LOG 过滤字符串 |
| **mcp.servers_dir** | string? | `null` | MCP 服务器配置目录 |
| **skills.dirs** | string[] | `[]` | 技能定义目录列表 |
| **working_dir** | string? | `null` | 沙箱工作目录 |
| **enable_event_bus** | bool | `false` | 启用事件总线（可观测性） |
| **tls.enabled** | bool | `false` | 启用 HTTPS |
| **tls.cert_path** | path? | `null` | PEM 证书路径 |
| **tls.key_path** | path? | `null` | PEM 私钥路径 |
| **tls.self_signed** | bool | `false` | 自动生成自签名证书 |
| **tls.self_signed_dir** | path? | `null` | 自签名证书输出目录 |
| **auth.mode** | string? | `null` | 认证模式：`none` / `api_key` |
| **auth.api_keys** | list? | `null` | API 密钥列表 |
| **scheduler.enabled** | bool | `false` | 启用定时任务调度器 |
| **scheduler.check_interval_secs** | u64 | `60` | 任务检查间隔（秒） |
| **scheduler.max_concurrent** | usize | `5` | 最大并发调度任务数 |
| **provider_chain** | object? | `null` | 多 Provider 故障转移配置 |
| **smart_routing** | object? | `null` | 复杂度路由配置 |
| **sync.enabled** | bool | `false` | 启用离线同步 |
| **sync.node_id** | string? | `null` | 节点标识（自动生成 UUID） |

### 3.3 环境变量参考

```bash
# 必需
ANTHROPIC_API_KEY=sk-ant-xxxxx     # Anthropic API 密钥
OPENAI_API_KEY=sk-xxxxx             # OpenAI API 密钥（使用 openai provider 时）

# 服务器
OCTO_HOST=127.0.0.1                 # 服务器绑定地址
OCTO_PORT=3001                      # 服务器端口
OCTO_DB_PATH=./data/octo.db        # 数据库路径
OCTO_CORS_ORIGINS=http://localhost:5180  # CORS 源（逗号分隔）

# 提供商
LLM_PROVIDER=anthropic              # 提供商选择
OPENAI_MODEL_NAME=gpt-4o            # 模型覆盖
ANTHROPIC_BASE_URL=                  # 代理 URL

# TLS
OCTO_TLS_ENABLED=false              # 启用 TLS
OCTO_TLS_CERT_PATH=                 # 证书路径
OCTO_TLS_KEY_PATH=                  # 私钥路径
OCTO_TLS_SELF_SIGNED=false          # 自签名证书

# 认证
OCTO_AUTH_MODE=none                 # 认证模式
OCTO_API_KEY=your-secret            # API 密钥
OCTO_API_KEY_USER=default           # 关联用户 ID

# 日志
RUST_LOG=octo_server=info,octo_engine=info

# 其他
OCTO_WORKING_DIR=./data/sandbox    # 沙箱目录
OCTO_ENABLE_EVENT_BUS=false        # 事件总线
```

---

## 4. Docker 部署

### 4.1 使用 Docker Compose（推荐）

```bash
# 创建 .env 文件
cp .env.example .env
# 编辑 .env，填入 ANTHROPIC_API_KEY

# 构建并启动
docker compose up -d

# 查看状态
docker compose ps

# 查看日志
docker compose logs -f octo-server
```

### 4.2 手动 Docker 构建

```bash
# 构建镜像
docker build -t octo-workbench:latest .

# 运行容器
docker run -d \
  --name octo-server \
  -p 3001:3001 \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -e OCTO_HOST=0.0.0.0 \
  -e RUST_LOG=info \
  -v $(pwd)/data:/app/data \
  octo-workbench:latest
```

### 4.3 Docker Compose 架构

```yaml
services:
  octo-server:        # Rust 后端 (端口 3001)
    image: octo-workbench:latest
    volumes:
      - ./data:/app/data          # 数据持久化
      - ./config.yaml:/app/config.yaml:ro  # 配置文件
    healthcheck:
      test: curl -f http://localhost:3001/api/health

  nginx:              # 前端静态服务 (端口 5180)
    image: nginx:alpine
    volumes:
      - ./docker/nginx.conf:/etc/nginx/conf.d/default.conf:ro
```

---

## 5. 反向代理配置

### 5.1 Caddy（推荐 — 自动 HTTPS）

```caddyfile
api.example.com {
    reverse_proxy localhost:3001
}
```

**优势**: Caddy 自动申请和续期 Let's Encrypt 证书，无需额外配置。

```bash
# 安装 Caddy: https://caddyserver.com/docs/install
# 启动
caddy run --config deploy/caddy/Caddyfile
```

### 5.2 Nginx

```nginx
server {
    listen 80;
    server_name api.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    # 安全头
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;

    location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket 支持
    location /ws {
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

```bash
# 安装 Certbot 并获取证书
sudo certbot --nginx -d api.example.com
```

完整配置参见 `deploy/nginx/nginx.conf`。

---

## 6. TLS 配置

### 6.1 方式一：反向代理终止 TLS（推荐）

使用 Caddy 或 Nginx + Certbot 在反向代理层终止 TLS，后端服务保持 HTTP。

这是生产环境的推荐方式，因为：
- 证书管理自动化（Caddy 或 Certbot）
- 后端无需处理 TLS 握手开销
- 配置更简洁

### 6.2 方式二：octo-server 内置 TLS

适用于无反向代理的场景（如直接暴露服务）。

**使用 PEM 证书文件**:

```bash
export OCTO_TLS_ENABLED=true
export OCTO_TLS_CERT_PATH=/path/to/cert.pem
export OCTO_TLS_KEY_PATH=/path/to/key.pem
```

**使用自签名证书（开发环境）**:

```bash
export OCTO_TLS_ENABLED=true
export OCTO_TLS_SELF_SIGNED=true
# 证书自动生成到 ./data/tls/ 目录
```

或在 `config.yaml` 中：

```yaml
tls:
  enabled: true
  self_signed: true
  self_signed_dir: "./data/tls"
```

---

## 7. 健康检查

### 7.1 HTTP 端点

```bash
curl http://localhost:3001/api/health
```

**正常响应** (HTTP 200):
```json
{
  "status": "ok"
}
```

### 7.2 Docker 健康检查

Dockerfile 和 docker-compose.yml 已内置健康检查配置：
- 检查间隔：30 秒
- 超时：10 秒
- 重试次数：3 次
- 启动等待：5-10 秒

### 7.3 外部监控集成

```bash
# 适用于 Prometheus/Blackbox Exporter
# 探测 /api/health 端点，HTTP 200 即为健康
```

---

## 8. 日志配置

### 8.1 日志级别

通过 `RUST_LOG` 环境变量控制：

```bash
# 生产环境（仅警告和错误）
RUST_LOG=warn

# 标准运行
RUST_LOG=info

# 调试模式
RUST_LOG=octo_server=debug,octo_engine=debug

# 详细调试（含 HTTP 层）
RUST_LOG=octo_server=debug,octo_engine=debug,tower_http=debug

# 特定模块调试
RUST_LOG=octo_engine::mcp=debug,octo_engine::agent=trace
```

### 8.2 日志格式

默认输出 pretty 格式到 stderr。在 Docker 中通过 `docker compose logs` 查看。

生产环境建议将 `RUST_LOG` 设为 `info` 或 `warn` 以减少日志量。

---

## 9. 备份策略

### 9.1 SQLite 数据库备份

octo-sandbox 使用 SQLite 作为持久化存储，数据库文件默认位于 `./data/octo.db`。

**手动备份**:
```bash
# 使用 SQLite 在线备份（推荐，不影响运行中的服务）
sqlite3 ./data/octo.db ".backup ./backups/octo-$(date +%Y%m%d-%H%M%S).db"
```

**定时备份（crontab）**:
```bash
# 每天凌晨 2 点备份
0 2 * * * sqlite3 /app/data/octo.db ".backup /app/backups/octo-$(date +\%Y\%m\%d).db"
```

**Docker 环境备份**:
```bash
# 从宿主机直接备份（volume 挂载到 ./data/）
sqlite3 ./data/octo.db ".backup ./backups/octo-backup.db"
```

### 9.2 备份保留策略

建议保留最近 7 天的每日备份和最近 4 周的周备份。

```bash
# 清理 30 天前的备份
find ./backups/ -name "octo-*.db" -mtime +30 -delete
```

---

## 10. 安全建议

### 10.1 认证配置

**生产环境必须启用认证**:

```bash
# API Key 模式
OCTO_AUTH_MODE=api_key
OCTO_API_KEY=your-strong-random-key-here
```

### 10.2 网络安全

- 后端服务绑定 `127.0.0.1`（不直接暴露到公网）
- 通过反向代理（Caddy/Nginx）暴露服务
- 启用 HTTPS（TLS）
- 配置 CORS 白名单：
  ```bash
  OCTO_CORS_ORIGINS=https://your-domain.com
  ```

### 10.3 安全清单

- [ ] 启用认证（`OCTO_AUTH_MODE=api_key`）
- [ ] 使用强随机 API Key
- [ ] 启用 HTTPS（通过反向代理或内置 TLS）
- [ ] 限制 CORS 源
- [ ] 绑定 127.0.0.1（不直接暴露）
- [ ] 定期备份数据库
- [ ] 设置日志级别为 `info` 或 `warn`
- [ ] 不在配置文件中硬编码密钥（使用 .env 或环境变量）
- [ ] 确保 `.env` 不被提交到版本控制
