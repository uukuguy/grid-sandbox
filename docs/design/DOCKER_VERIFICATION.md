# Docker 部署验证清单

> 本文档提供 Docker 构建和运行验证步骤，由用户在实际环境中手动执行。

---

## 1. 前置条件

- Docker Engine 20.10+ 已安装
- Docker Compose v2 已安装
- 项目根目录下存在 `.env` 文件（包含 `ANTHROPIC_API_KEY`）
- 确保端口 3001 和 5180 未被占用

```bash
docker --version       # 预期: Docker version 20.10+
docker compose version # 预期: Docker Compose version v2.x
```

---

## 2. 构建镜像

### 2.1 单独构建

```bash
docker build -t octo-workbench:latest .
```

**预期结果**:
- 构建成功，输出 `Successfully built` 或 `Successfully tagged octo-workbench:latest`
- 三阶段构建完成：Rust builder -> Frontend builder -> Production

**失败排查**:
- `error: linker 'cc' not found` — builder 阶段缺少 build-essential，检查 Dockerfile
- `pnpm install` 失败 — 检查 `web/pnpm-lock.yaml` 是否存在
- `cargo build` 失败 — 检查 Cargo.lock 是否最新（运行 `cargo check --workspace` 确认本地能编译）

### 2.2 通过 Compose 构建

```bash
docker compose build
```

**预期结果**: 同 2.1，但通过 docker-compose.yml 配置构建。

---

## 3. 启动服务

```bash
docker compose up -d
```

**预期结果**:
- `octo-workbench-server` 容器启动（后端 API）
- `octo-workbench-web` 容器启动（Nginx 前端代理）
- 两个容器都处于 `Up` 状态

```bash
docker compose ps
```

**预期输出**:
```
NAME                    STATUS          PORTS
octo-workbench-server   Up (healthy)    0.0.0.0:3001->3001/tcp
octo-workbench-web      Up              0.0.0.0:5180->80/tcp
```

---

## 4. 健康检查验证

### 4.1 后端健康检查

```bash
curl -f http://localhost:3001/api/health
```

**预期输出**: HTTP 200，JSON 响应包含 `"status": "ok"`

```json
{
  "status": "ok"
}
```

### 4.2 前端访问验证

```bash
curl -s -o /dev/null -w "%{http_code}" http://localhost:5180
```

**预期输出**: `200`

### 4.3 Docker 内置健康检查

```bash
docker inspect --format='{{.State.Health.Status}}' octo-workbench-server
```

**预期输出**: `healthy`

---

## 5. 数据持久化验证

```bash
# 检查数据卷挂载
docker compose exec octo-server ls -la /app/data/

# 预期: 存在 octo.db 文件
```

### 5.1 重启后数据保留

```bash
docker compose restart
# 等待服务就绪后再次检查
curl http://localhost:3001/api/health
```

**预期**: 数据在重启后保留（SQLite 文件通过 volume 挂载到宿主机 `./data/`）。

---

## 6. 日志查看

```bash
# 查看后端日志
docker compose logs octo-server --tail=50

# 查看前端日志
docker compose logs nginx --tail=50

# 实时跟踪
docker compose logs -f octo-server
```

---

## 7. 停止和清理

```bash
# 停止服务
docker compose down

# 停止并删除数据卷（谨慎操作）
docker compose down -v

# 清理构建缓存
docker builder prune
```

---

## 8. 常见问题排查

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| 构建时 Rust 编译超时 | 依赖下载慢 | 配置 cargo registry mirror 或增加 Docker 内存 |
| 容器启动后立即退出 | 缺少 ANTHROPIC_API_KEY | 在 `.env` 中配置 API Key |
| 端口 3001 被占用 | 本地服务冲突 | 修改 docker-compose.yml 的 ports 映射 |
| 前端 502 Bad Gateway | 后端未就绪 | 检查 octo-server 容器状态和日志 |
| SQLite 权限错误 | Volume 权限不匹配 | 检查 `./data/` 目录权限，确保容器内用户可写 |
| `curl: (7) Failed to connect` | 服务未绑定 0.0.0.0 | 确认 `OCTO_HOST=0.0.0.0` 环境变量已设置 |
| Nginx 配置缺失 | `docker/nginx.conf` 不存在 | 创建 `docker/nginx.conf` 或参考 `deploy/nginx/nginx.conf` |

---

## 9. 架构说明

```
                    +-------------------+
    :5180 --------> |  Nginx (Alpine)   | ---+
                    +-------------------+    |
                                             | proxy_pass
                    +-------------------+    |
    :3001 --------> | octo-server (Rust)| <--+
                    +-------------------+
                            |
                    +-------v-----------+
                    |  ./data/octo.db   |  (Volume 挂载)
                    +-------------------+
```

- **Nginx**: 提供前端静态文件服务，反向代理 API 请求到后端
- **octo-server**: Rust 后端，处理 API 和 WebSocket
- **数据卷**: SQLite 数据库和配置文件通过 volume 持久化到宿主机
