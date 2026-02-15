# OpenERP ACL 权限规则

> 完整的权限定义、检查流程、root 特殊处理

## 权限模型

```
User ──member_of──→ Group ──has──→ Policy ──grants──→ Role ──contains──→ Permission
                      │                                                      │
                      └── parent_id ──→ Group (层级继承)          module:resource:action
```

### 核心概念

| 概念 | 说明 | 示例 |
|------|------|------|
| **Permission** | 最小权限单元 | `pms:device:create` |
| **Role** | 一组 permissions | `pms:admin` = `[pms:device:*, pms:batch:*]` |
| **Policy** | 授权记录 (who + what + how) | `user:alice` + `pms:device` + `pms:admin` |
| **Group** | 组织层级，Policy 可以挂在 Group 上 | `engineering` → 继承到所有成员 |

### Permission 格式

```
module:resource:action
  │       │       │
  │       │       └── create / read / update / delete / list / 自定义
  │       └── 资源类型（snake_case）
  └── 模块名
```

**通配符：**
- `pms:device:*` — device 的所有操作
- `pms:*` — pms 模块的所有操作（不推荐）
- `*` — 所有权限（仅 auth:root 使用）

---

## 完整权限清单

### Auth 模块

| 资源 | Permission | 说明 |
|------|-----------|------|
| **User** | `auth:user:create` | 创建用户 |
| | `auth:user:read` | 查看用户详情 |
| | `auth:user:update` | 修改用户信息 |
| | `auth:user:delete` | 删除用户 |
| | `auth:user:list` | 列出用户 |
| **Session** | `auth:session:create` | 创建会话（登录） |
| | `auth:session:read` | 查看会话详情 |
| | `auth:session:delete` | 删除会话 |
| | `auth:session:list` | 列出会话 |
| | `auth:session:revoke` | 撤销会话 |
| **Role** | `auth:role:create` | 创建角色 |
| | `auth:role:read` | 查看角色 |
| | `auth:role:update` | 修改角色 |
| | `auth:role:delete` | 删除角色 |
| | `auth:role:list` | 列出角色 |
| **Group** | `auth:group:create` | 创建组 |
| | `auth:group:read` | 查看组 |
| | `auth:group:update` | 修改组 |
| | `auth:group:delete` | 删除组 |
| | `auth:group:list` | 列出组 |
| | `auth:group:add_member` | 添加成员 |
| | `auth:group:remove_member` | 移除成员 |
| **Policy** | `auth:policy:create` | 创建策略 |
| | `auth:policy:read` | 查看策略 |
| | `auth:policy:update` | 修改策略 |
| | `auth:policy:delete` | 删除策略 |
| | `auth:policy:list` | 列出策略 |
| | `auth:policy:check` | 检查权限 |
| **Provider** | `auth:provider:create` | 创建 OAuth 提供商 |
| | `auth:provider:read` | 查看提供商 |
| | `auth:provider:update` | 修改提供商 |
| | `auth:provider:delete` | 删除提供商 |
| | `auth:provider:list` | 列出提供商 |

### PMS 模块

| 资源 | Permission | 说明 |
|------|-----------|------|
| **Device** | `pms:device:create` | 创建设备 |
| | `pms:device:read` | 查看设备 |
| | `pms:device:update` | 修改设备 |
| | `pms:device:delete` | 删除设备 |
| | `pms:device:list` | 列出设备 |
| | `pms:device:provision` | 配置设备 |
| | `pms:device:activate` | 激活设备 |
| **Batch** | `pms:batch:create` | 创建批次 |
| | `pms:batch:read` | 查看批次 |
| | `pms:batch:update` | 修改批次 |
| | `pms:batch:delete` | 删除批次 |
| | `pms:batch:list` | 列出批次 |
| | `pms:batch:provision` | 执行批次配置 |
| **License** | `pms:license:create` | 创建许可证 |
| | `pms:license:read` | 查看许可证 |
| | `pms:license:update` | 修改许可证 |
| | `pms:license:delete` | 删除许可证 |
| | `pms:license:list` | 列出许可证 |
| **Firmware** | `pms:firmware:create` | 创建固件记录 |
| | `pms:firmware:read` | 查看固件 |
| | `pms:firmware:update` | 修改固件 |
| | `pms:firmware:delete` | 删除固件 |
| | `pms:firmware:list` | 列出固件 |
| | `pms:firmware:upload` | 上传固件文件 |
| **Model** | `pms:model:create` | 创建型号 |
| | `pms:model:read` | 查看型号 |
| | `pms:model:update` | 修改型号 |
| | `pms:model:delete` | 删除型号 |
| | `pms:model:list` | 列出型号 |
| **Segment** | `pms:segment:create` | 创建 SN 段号 |
| | `pms:segment:read` | 查看段号 |
| | `pms:segment:update` | 修改段号 |
| | `pms:segment:delete` | 删除段号 |
| | `pms:segment:list` | 列出段号 |
| **LicenseImport** | `pms:license_import:create` | 创建导入记录 |
| | `pms:license_import:read` | 查看导入记录 |
| | `pms:license_import:list` | 列出导入记录 |
| | `pms:license_import:import` | 执行批量导入 |

### Task 模块

| 资源 | Permission | 说明 |
|------|-----------|------|
| **Task** | `task:task:create` | 创建任务 |
| | `task:task:read` | 查看任务 |
| | `task:task:list` | 列出任务 |
| | `task:task:cancel` | 取消任务 |
| | `task:task:claim` | 认领任务（executor） |
| | `task:task:progress` | 报告进度（executor） |
| | `task:task:complete` | 完成任务（executor） |
| | `task:task:fail` | 标记失败（executor） |
| | `task:task:log` | 写入日志（executor） |
| | `task:task:poll` | 长轮询等待变更 |
| **TaskType** | `task:task_type:create` | 注册任务类型 |
| | `task:task_type:read` | 查看任务类型 |
| | `task:task_type:update` | 修改任务类型 |
| | `task:task_type:delete` | 删除任务类型 |
| | `task:task_type:list` | 列出任务类型 |

---

## Root 账号

### 设计

- **虚拟账号**：数据库中没有 User record
- **密码存储**：argon2id hash 存在服务端配置文件 `[root].password_hash`
- **Role**：`auth:root`（启动时自动创建在数据库中）
- **JWT**：`{ sub: "root", name: "Root", roles: ["auth:root"], groups: [] }`

### auth:root Role

```json
{
  "id": "auth:root",
  "description": "Superadmin — bypasses all permission checks",
  "permissions": [],
  "service": "auth"
}
```

`permissions` 为空！权限检查在代码中特殊处理，不依赖 permission 列表。

### 权限检查中的 Root 处理

```rust
pub fn check_permission(claims: &Claims, permission: &str) -> Result<(), ApiError> {
    // Root 直接放行
    if claims.roles.contains(&"auth:root".to_string()) {
        return Ok(());
    }
    
    // 正常用户走 Policy 系统
    policy_service.check_permission(&CheckParams {
        who: format!("user:{}", claims.sub),
        what: build_what_path(permission),  // "pms:device" 或 "pms:device:HVV-123"
        how: permission.to_string(),        // "pms:device:create"
    })?;
    
    Ok(())
}
```

---

## 权限检查流程

### 完整流程图

```
HTTP Request
  │
  ├── 提取 Authorization: Bearer <JWT>
  │     └── 验证 JWT 签名 + 过期时间
  │
  ├── 解析 Claims { sub, roles, groups }
  │
  ├── Root 检查
  │     └── roles 包含 "auth:root"? → Yes → 直接放行
  │                                  → No  → 继续
  │
  ├── 构建 who: "user:{sub}"
  │
  ├── 展开 groups → 所有祖先 groups
  │     └── 构建 identities: ["user:alice", "group:eng", "group:company"]
  │
  ├── 构建 what 路径（从具体到全局）
  │     └── "pms:device:HVV-123" → ["pms:device:HVV-123", "pms:device", "pms", ""]
  │
  ├── 查询所有匹配 Policies
  │     └── SELECT * FROM policies WHERE who IN (identities)
  │
  ├── 对每个 Policy:
  │     ├── 检查过期时间
  │     ├── 检查 what 路径匹配
  │     └── 展开 Role → 检查是否包含所需 permission
  │           ├── 精确匹配: "pms:device:create" == "pms:device:create"
  │           └── 通配符: "pms:device:*" matches "pms:device:create"
  │
  └── 有匹配 → 放行
      无匹配 → 403 Forbidden
```

### DSL 中的权限声明

```rust
// 权限定义在端点级别
#[api(Device)]
impl DeviceApi {
    #[endpoint(POST "/devices")]
    #[permission("pms:device:create")]
    async fn create(body: CreateDeviceRequest) -> Device;
    
    #[endpoint(GET "/devices/:sn")]
    #[permission("pms:device:read")]
    async fn get(sn: String) -> Device;
    
    #[endpoint(POST "/devices/:sn/@provision")]
    #[permission("pms:device:provision")]
    #[handler = "provision"]
    async fn provision(sn: String, body: ProvisionRequest) -> Device;
}
```

生成的 handler 自动插入权限检查：

```rust
// 自动生成
async fn create_handler(
    claims: Claims,  // 从 JWT 提取
    State(svc): State<Arc<DeviceService>>,
    Json(body): Json<CreateDeviceRequest>,
) -> Result<Json<Device>, ApiError> {
    // 自动插入的权限检查
    check_permission(&claims, "pms:device:create")?;
    
    // 业务逻辑
    let device = svc.create(body).await?;
    Ok(Json(device))
}
```

---

## 预定义角色

### 推荐的初始角色配置

```json
[
  {
    "id": "auth:root",
    "description": "Superadmin — bypasses all permission checks",
    "permissions": [],
    "service": "auth"
  },
  {
    "id": "auth:admin",
    "description": "Auth module administrator",
    "permissions": ["auth:*"],
    "service": "auth"
  },
  {
    "id": "pms:admin",
    "description": "PMS module administrator",
    "permissions": ["pms:*"],
    "service": "pms"
  },
  {
    "id": "pms:operator",
    "description": "PMS operator — can provision and manage devices",
    "permissions": [
      "pms:device:read", "pms:device:list", "pms:device:provision", "pms:device:activate",
      "pms:batch:read", "pms:batch:list", "pms:batch:provision",
      "pms:license:read", "pms:license:list",
      "pms:firmware:read", "pms:firmware:list"
    ],
    "service": "pms"
  },
  {
    "id": "pms:viewer",
    "description": "PMS read-only access",
    "permissions": [
      "pms:device:read", "pms:device:list",
      "pms:batch:read", "pms:batch:list",
      "pms:license:read", "pms:license:list",
      "pms:firmware:read", "pms:firmware:list",
      "pms:model:read", "pms:model:list",
      "pms:segment:read", "pms:segment:list"
    ],
    "service": "pms"
  },
  {
    "id": "task:executor",
    "description": "Task executor — can claim, report, complete tasks",
    "permissions": [
      "task:task:read", "task:task:list",
      "task:task:claim", "task:task:progress", "task:task:complete", "task:task:fail",
      "task:task:log", "task:task:poll"
    ],
    "service": "task"
  }
]
```

---

## 公开端点（不需要认证）

以下端点不需要 JWT，任何人可以访问：

| 端点 | 说明 |
|------|------|
| `POST /auth/login` | 登录（root 密码 / OAuth） |
| `GET /auth/providers` | 列出可用的 OAuth 提供商（公开信息） |
| `GET /auth/oauth/:provider/authorize` | OAuth 重定向 |
| `GET /auth/oauth/:provider/callback` | OAuth 回调 |
| `POST /auth/token/refresh` | 刷新 JWT |
| `GET /health` | 健康检查 |
| `GET /version` | 版本信息 |
| `GET /` | 登录页面（HTML） |
