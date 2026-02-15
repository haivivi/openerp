# OpenERP CLI Reference

> `openerp` — 客户端命令行工具，管理 contexts、认证、资源操作

## 概述

```
openerp <command> [subcommand] [flags]
```

OpenERP 使用 **context** 概念管理多个服务实例（类似 kubectl）。每个 context 对应一个 openerpd server 实例。

## 全局 Flags

```
--config <path>     客户端配置文件路径（默认 ~/.openerp/config.toml）
--context <name>    临时指定 context（不修改 current-context）
--output <format>   输出格式：table（默认）、json、yaml
--verbose           显示详细日志
```

---

## Context 管理

### `openerp context create <name>`

创建新的 context（生成服务端配置文件 + 设置 root 密码）。

```bash
$ openerp context create cn-stage
Config directory [/etc/openerp]: 
Data directory [/var/lib/openerp/cn-stage]: 
Enter root password: ****
Confirm root password: ****
Context "cn-stage" created.
  Config: /etc/openerp/cn-stage.toml
  Data:   /var/lib/openerp/cn-stage/
```

**Flags:**
```
--config-dir <path>   服务端配置目录（默认 /etc/openerp/）
--data-dir <path>     数据存储目录（默认 /var/lib/openerp/<name>/）
```

**生成的文件：**

`/etc/openerp/cn-stage.toml`：
```toml
[root]
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$随机salt$hash"

[storage]
data_dir = "/var/lib/openerp/cn-stage"

[jwt]
secret = "自动生成的随机字符串"
expire_secs = 86400
```

`~/.openerp/config.toml`（追加）：
```toml
[[contexts]]
name = "cn-stage"
config_path = "/etc/openerp/cn-stage.toml"
server = ""
token = ""
```

### `openerp context list`

列出所有 contexts。

```bash
$ openerp context list
  NAME        SERVER                    STATUS
* cn-stage    http://localhost:8080      connected
  us-prod     https://us.example.com    disconnected
  local       http://localhost:9090      -
```

`*` 标记当前 context。

### `openerp use context <name>`

切换当前 context。

```bash
$ openerp use context us-prod
Switched to context "us-prod".
```

### `openerp context set <name> --server <url>`

更新 context 的 server URL（连接远程 openerpd 时用）。

```bash
$ openerp context set cn-stage --server https://cn-stage.openerp.example.com
Context "cn-stage" updated.
```

### `openerp context delete <name>`

删除 context（不删除服务端配置文件）。

```bash
$ openerp context delete local
Context "local" deleted.
```

---

## Root 管理

### `openerp root chpwd`

修改当前 context 的 root 密码。

```bash
$ openerp root chpwd
Current root password: ****
New root password: ****
Confirm new password: ****
Root password updated for context "cn-stage".
```

**注意：** 需要重启 openerpd 才能生效（openerpd 启动时加载配置）。

### `openerp root chpwd --context <name>`

修改指定 context 的 root 密码。

```bash
$ openerp root chpwd --context us-prod
```

---

## 认证

### `openerp login`

登录当前 context 的 server。

```bash
# Root 登录（密码认证）
$ openerp login
Username: root
Password: ****
Logged in as root.
Token saved to context "cn-stage".

# 指定用户
$ openerp login --user alice
Password: ****
Logged in as alice.
```

**Flags:**
```
--user <name>    用户名（默认交互式输入）
--password <pw>  密码（不推荐，用于脚本）
```

### `openerp logout`

清除当前 context 的 token。

```bash
$ openerp logout
Logged out from context "cn-stage".
```

---

## 资源操作

### `openerp get <resource> [id]`

查询资源。

```bash
# 列出资源
$ openerp get users
ID          NAME           EMAIL              ACTIVE
abc123      Alice          alice@example.com   true
def456      Bob            bob@example.com     true

# 查看单个资源
$ openerp get user abc123
ID:       abc123
Name:     Alice
Email:    alice@example.com
Active:   true
Created:  2026-02-13T10:00:00Z

# JSON 输出
$ openerp get user abc123 --output json
{"id":"abc123","name":"Alice",...}
```

**支持的资源类型：**

| 单数 | 复数 | 模块 |
|------|------|------|
| user | users | auth |
| session | sessions | auth |
| role | roles | auth |
| group | groups | auth |
| policy | policies | auth |
| provider | providers | auth |
| device | devices | pms |
| batch | batches | pms |
| license | licenses | pms |
| firmware | firmwares | pms |
| model | models | pms |
| segment | segments | pms |
| license-import | license-imports | pms |
| task | tasks | task |
| task-type | task-types | task |

**Flags:**
```
--limit <n>       分页大小（默认 20）
--offset <n>      分页偏移
--sort <field>    排序字段（前缀 - 表示降序）
--filter <expr>   过滤表达式
```

### `openerp create <resource> [flags | json]`

创建资源。

```bash
# 交互式（未来支持）
$ openerp create user --name Alice --email alice@example.com

# JSON 输入
$ openerp create provider --json '{
  "id": "github",
  "name": "GitHub",
  "provider_type": "oauth2",
  "client_id": "Iv1.xxx",
  "client_secret": "xxx",
  "auth_url": "https://github.com/login/oauth/authorize",
  "token_url": "https://github.com/login/oauth/access_token",
  "userinfo_url": "https://api.github.com/user",
  "scopes": ["user:email"],
  "redirect_url": "http://localhost:8080/auth/callback/github"
}'

# 从文件
$ openerp create provider -f github-provider.json
```

### `openerp update <resource> <id> [flags | json]`

更新资源（JSON merge-patch）。

```bash
$ openerp update user abc123 --json '{"name": "Alice Smith"}'
User abc123 updated.
```

### `openerp delete <resource> <id>`

删除资源。

```bash
$ openerp delete user abc123
Are you sure? [y/N]: y
User abc123 deleted.

# 跳过确认
$ openerp delete user abc123 --yes
```

---

## 状态检查

### `openerp version`

```bash
$ openerp version
openerp cli v0.1.0
```

### `openerp status`

检查当前 context 的 server 连接。

```bash
$ openerp status
Context:   cn-stage
Server:    http://localhost:8080
Status:    connected
Version:   openerpd v0.1.0
Uptime:    2h 15m
User:      root (auth:root)
```

---

## 配置文件

### 客户端配置 `~/.openerp/config.toml`

```toml
current-context = "cn-stage"

[[contexts]]
name = "cn-stage"
config_path = "/etc/openerp/cn-stage.toml"
server = "http://localhost:8080"
token = "eyJhbGciOiJIUzI1NiIs..."

[[contexts]]
name = "us-prod"
config_path = ""
server = "https://us.openerp.example.com"
token = "eyJhbGciOiJIUzI1NiIs..."
```

**字段说明：**
- `config_path`：服务端配置文件路径（本地部署时填，用于 `root chpwd`）
- `server`：server URL（远程连接时填）
- `token`：JWT token（`openerp login` 后自动填充）

### 服务端配置 `/etc/openerp/<name>.toml`

```toml
[root]
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$salt$hash"

[storage]
data_dir = "/var/lib/openerp/cn-stage"

[jwt]
secret = "random-256-bit-secret"
expire_secs = 86400
```

---

## 典型工作流

### 首次部署

```bash
# 1. 创建 context（设置 root 密码）
$ openerp context create cn-stage

# 2. 启动 server
$ openerpd -c cn-stage --listen 0.0.0.0:8080

# 3. 设置 server URL
$ openerp context set cn-stage --server http://localhost:8080

# 4. 用 root 登录
$ openerp login --user root

# 5. 配置 OAuth Provider
$ openerp create provider -f github.json

# 6. 创建管理员角色
$ openerp create role --json '{
  "id": "pms:admin",
  "permissions": ["pms:device:*", "pms:batch:*"],
  "service": "pms"
}'
```

### 日常使用

```bash
$ openerp use context cn-stage
$ openerp get devices --limit 10 --sort -create_at
$ openerp get device HVV-A1B2C
```

### 多环境

```bash
# 连接远程 server（不需要本地配置文件）
$ openerp context set us-prod --server https://us.openerp.example.com
$ openerp use context us-prod
$ openerp login --user root
$ openerp get users
```
