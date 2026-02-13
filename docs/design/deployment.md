# OpenERP 部署指南

## 构建

```bash
# 构建 openerpd (server)
bazel build //rust/bin/openerpd

# 构建 openerp (CLI)
bazel build //rust/bin/openerp

# 构建全部
bazel build //...

# 运行测试
bazel test //...
```

构建产物：
```
bazel-bin/rust/bin/openerpd/openerpd
bazel-bin/rust/bin/openerp/openerp
```

## 首次安装

### 1. 创建 Context

```bash
# 安装 openerp CLI 到 PATH
cp bazel-bin/rust/bin/openerp/openerp /usr/local/bin/

# 创建服务实例配置
openerp context create cn-stage
```

交互式输入：
```
Config directory [/etc/openerp]:
Data directory [/var/lib/openerp/cn-stage]:
Enter root password: ****
Confirm root password: ****
```

生成文件：
- `/etc/openerp/cn-stage.toml` — 服务端配置（含 root 密码 hash）
- `/var/lib/openerp/cn-stage/` — 数据目录
- `~/.openerp/config.toml` — 客户端配置

### 2. 启动 Server

```bash
# 直接运行
openerpd -c cn-stage --listen 0.0.0.0:8080

# 或通过 bazel 运行
bazel run //rust/bin/openerpd -- -c cn-stage --listen 0.0.0.0:8080
```

### 3. 配置 CLI 连接

```bash
openerp context set cn-stage --server http://localhost:8080
```

### 4. Root 登录

```bash
openerp login --user root
# 输入安装时设置的密码
```

### 5. 基础配置

```bash
# 创建管理员角色
openerp create role --json '{
  "id": "pms:admin",
  "permissions": ["pms:*"],
  "service": "pms"
}'

# 配置 OAuth Provider
openerp create provider --json '{
  "id": "github",
  "name": "GitHub",
  "provider_type": "oauth2",
  "client_id": "YOUR_CLIENT_ID",
  "client_secret": "YOUR_CLIENT_SECRET",
  "auth_url": "https://github.com/login/oauth/authorize",
  "token_url": "https://github.com/login/oauth/access_token",
  "userinfo_url": "https://api.github.com/user",
  "scopes": ["user:email"],
  "redirect_url": "http://localhost:8080/auth/callback/github"
}'
```

## Systemd 服务

创建 `/etc/systemd/system/openerpd.service`：

```ini
[Unit]
Description=OpenERP Server
After=network.target

[Service]
Type=simple
User=openerp
Group=openerp
ExecStart=/usr/local/bin/openerpd -c cn-stage --listen 0.0.0.0:8080
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable openerpd
sudo systemctl start openerpd
sudo journalctl -u openerpd -f
```

## 多环境

```bash
# 创建另一个 context
openerp context create us-prod --data-dir /var/lib/openerp/us-prod

# 在不同端口启动
openerpd -c us-prod --listen 0.0.0.0:8081

# CLI 切换
openerp context set us-prod --server http://us-prod.example.com:8081
openerp use context us-prod
openerp login --user root
```

## Root 密码管理

```bash
# 修改 root 密码
openerp root chpwd
# 输入当前密码和新密码

# 修改指定 context 的 root 密码
openerp root chpwd --context us-prod

# 注意：修改密码后需要重启 openerpd
sudo systemctl restart openerpd
```

## 目录结构

```
/etc/openerp/
├── cn-stage.toml          # 服务端配置
└── us-prod.toml

/var/lib/openerp/
├── cn-stage/              # 数据目录
│   ├── data.redb          # KV 存储
│   ├── data.sqlite        # SQL 存储
│   ├── search/            # 全文索引
│   ├── blobs/             # 文件存储
│   └── tsdb/              # 时序数据
└── us-prod/

~/.openerp/
└── config.toml            # CLI 客户端配置
```

## API 端点清单

### 公开端点（不需要认证）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 登录页面 |
| GET | `/health` | 健康检查 |
| GET | `/version` | 版本信息 |
| POST | `/auth/login` | 登录（root / OAuth） |
| GET | `/auth/providers` | 可用的 OAuth 提供商 |

### Auth 模块 (`/auth`)

| 方法 | 路径 | Permission |
|------|------|-----------|
| POST | `/auth/users` | `auth:user:create` |
| GET | `/auth/users` | `auth:user:list` |
| GET | `/auth/users/:id` | `auth:user:read` |
| PATCH | `/auth/users/:id` | `auth:user:update` |
| DELETE | `/auth/users/:id` | `auth:user:delete` |
| GET | `/auth/me` | `auth:user:read` |
| GET | `/auth/sessions` | `auth:session:list` |
| GET | `/auth/sessions/:id` | `auth:session:read` |
| DELETE | `/auth/sessions/:id` | `auth:session:delete` |
| POST | `/auth/sessions/:id/@revoke` | `auth:session:revoke` |
| POST | `/auth/roles` | `auth:role:create` |
| GET | `/auth/roles` | `auth:role:list` |
| GET | `/auth/roles/:id` | `auth:role:read` |
| PATCH | `/auth/roles/:id` | `auth:role:update` |
| DELETE | `/auth/roles/:id` | `auth:role:delete` |
| POST | `/auth/groups` | `auth:group:create` |
| GET | `/auth/groups` | `auth:group:list` |
| GET | `/auth/groups/:id` | `auth:group:read` |
| PATCH | `/auth/groups/:id` | `auth:group:update` |
| DELETE | `/auth/groups/:id` | `auth:group:delete` |
| GET | `/auth/groups/:id/@members` | `auth:group:read` |
| POST | `/auth/groups/:id/@members` | `auth:group:add_member` |
| DELETE | `/auth/groups/:id/@members/:ref` | `auth:group:remove_member` |
| POST | `/auth/policies` | `auth:policy:create` |
| GET | `/auth/policies` | `auth:policy:list` |
| GET | `/auth/policies/:id` | `auth:policy:read` |
| PATCH | `/auth/policies/:id` | `auth:policy:update` |
| DELETE | `/auth/policies/:id` | `auth:policy:delete` |
| POST | `/auth/check` | `auth:policy:check` |
| POST | `/auth/providers` | `auth:provider:create` |
| GET | `/auth/providers` | `auth:provider:list` |
| GET | `/auth/providers/:id` | `auth:provider:read` |
| PATCH | `/auth/providers/:id` | `auth:provider:update` |
| DELETE | `/auth/providers/:id` | `auth:provider:delete` |

### PMS 模块 (`/pms`)

| 方法 | 路径 | Permission |
|------|------|-----------|
| POST | `/pms/devices` | `pms:device:create` |
| GET | `/pms/devices` | `pms:device:list` |
| GET | `/pms/devices/:sn` | `pms:device:read` |
| PATCH | `/pms/devices/:sn` | `pms:device:update` |
| DELETE | `/pms/devices/:sn` | `pms:device:delete` |
| POST | `/pms/devices/:sn/@provision` | `pms:device:provision` |
| POST | `/pms/devices/:sn/@activate` | `pms:device:activate` |
| POST | `/pms/batches` | `pms:batch:create` |
| GET | `/pms/batches` | `pms:batch:list` |
| GET | `/pms/batches/:id` | `pms:batch:read` |
| PATCH | `/pms/batches/:id` | `pms:batch:update` |
| DELETE | `/pms/batches/:id` | `pms:batch:delete` |
| POST | `/pms/batches/:id/@provision` | `pms:batch:provision` |
| POST | `/pms/licenses` | `pms:license:create` |
| GET | `/pms/licenses` | `pms:license:list` |
| GET | `/pms/licenses/:id` | `pms:license:read` |
| PATCH | `/pms/licenses/:id` | `pms:license:update` |
| DELETE | `/pms/licenses/:id` | `pms:license:delete` |
| POST | `/pms/firmwares` | `pms:firmware:create` |
| GET | `/pms/firmwares` | `pms:firmware:list` |
| GET | `/pms/firmwares/:id` | `pms:firmware:read` |
| PATCH | `/pms/firmwares/:id` | `pms:firmware:update` |
| DELETE | `/pms/firmwares/:id` | `pms:firmware:delete` |
| POST | `/pms/firmwares/:id/@upload` | `pms:firmware:upload` |
| POST | `/pms/models` | `pms:model:create` |
| GET | `/pms/models` | `pms:model:list` |
| GET | `/pms/models/:code` | `pms:model:read` |
| PATCH | `/pms/models/:code` | `pms:model:update` |
| DELETE | `/pms/models/:code` | `pms:model:delete` |
| POST | `/pms/segments` | `pms:segment:create` |
| GET | `/pms/segments` | `pms:segment:list` |
| GET | `/pms/segments/:id` | `pms:segment:read` |
| PATCH | `/pms/segments/:id` | `pms:segment:update` |
| DELETE | `/pms/segments/:id` | `pms:segment:delete` |
| POST | `/pms/license-imports` | `pms:license_import:create` |
| GET | `/pms/license-imports` | `pms:license_import:list` |
| GET | `/pms/license-imports/:id` | `pms:license_import:read` |
| POST | `/pms/license-imports/:id/@import` | `pms:license_import:import` |

### Task 模块 (`/task`)

| 方法 | 路径 | Permission |
|------|------|-----------|
| POST | `/task/tasks` | `task:task:create` |
| GET | `/task/tasks` | `task:task:list` |
| GET | `/task/tasks/:id` | `task:task:read` |
| POST | `/task/tasks/:id/@claim` | `task:task:claim` |
| POST | `/task/tasks/:id/@progress` | `task:task:progress` |
| POST | `/task/tasks/:id/@complete` | `task:task:complete` |
| POST | `/task/tasks/:id/@fail` | `task:task:fail` |
| POST | `/task/tasks/:id/@cancel` | `task:task:cancel` |
| GET | `/task/tasks/:id/@poll` | `task:task:poll` |
| POST | `/task/tasks/:id/@log` | `task:task:log` |
| GET | `/task/tasks/:id/@logs` | `task:task:read` |
| POST | `/task/task-types` | `task:task_type:create` |
| GET | `/task/task-types` | `task:task_type:list` |
| GET | `/task/task-types/:id` | `task:task_type:read` |
| DELETE | `/task/task-types/:id` | `task:task_type:delete` |
