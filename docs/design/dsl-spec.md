# OpenERP DSL 规范

> 资源定义语法、attribute 参考、实现分离规则

## 概述

OpenERP DSL 使用 Rust attribute macros 定义资源。DSL 文件本身就是合法的 Rust 代码，可以直接 import 其他库。

**三种资源类型：**

| 类型 | 用途 | 生成 |
|------|------|------|
| `#[model]` | 纯数据模型（枚举、嵌套结构） | 无 API |
| `#[db_resource]` | 数据库资源 | 自动 CRUD（5 个端点） |
| `#[api]` | 自定义端点 | 用户定义的端点 |

---

## 文件组织

### DSL 定义

```
rust/lib/api/schema/
├── auth/
│   ├── mod.rs          # pub mod user; pub mod role; ...
│   ├── user.rs         # User 资源定义
│   ├── session.rs
│   ├── role.rs
│   ├── group.rs
│   ├── policy.rs
│   └── provider.rs
├── pms/
│   ├── mod.rs
│   ├── device.rs
│   └── ...
└── task/
    ├── mod.rs
    ├── task.rs
    └── task_type.rs
```

### 实现文件

```
rust/mod/{module}/src/handlers/
├── {resource}/
│   ├── {action}.rs      # 每个自定义操作一个文件
│   └── ...
└── ...
```

例如：

```
rust/mod/pms/src/handlers/
├── device/
│   ├── provision.rs     # POST /devices/:sn/@provision
│   └── activate.rs      # POST /devices/:sn/@activate
└── batch/
    └── provision.rs     # POST /batches/:id/@provision
```

---

## `#[model]` — 纯数据模型

用于枚举、嵌套结构体等不需要 API 的类型。

```rust
#[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceStatus {
    Pending,
    Provisioned,
    Activated,
    Retired,
}

#[model]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub country: String,
}
```

**生成：**
- TypeScript enum / interface
- 无 API、无数据库表

---

## `#[db_resource]` — 数据库资源

自动生成完整 CRUD（5 个端点 + SQL + Service + 前端）。

### 基本语法

```rust
#[db_resource(
    module = "pms",           // 模块名（必填）
    table = "devices",        // 数据库表名（必填）
    display_name = "Device",  // UI 显示名（可选，默认 struct name）
)]
#[permission(create = "pms:device:create")]
#[permission(read = "pms:device:read")]
#[permission(update = "pms:device:update")]
#[permission(delete = "pms:device:delete")]
#[permission(list = "pms:device:list")]
pub struct Device {
    #[primary_key]
    pub sn: String,
    
    pub secret: String,
    pub model: u32,
    
    #[default(DeviceStatus::Pending)]
    pub status: DeviceStatus,
    
    pub sku: Option<String>,
    
    #[auto_timestamp(on_create)]
    pub create_at: Option<String>,
    
    #[auto_timestamp(on_update)]
    pub update_at: Option<String>,
}
```

### 自动生成的端点

| 方法 | 路径 | Permission |
|------|------|-----------|
| POST | `/{module}/{table}` | `create` |
| GET | `/{module}/{table}/:id` | `read` |
| GET | `/{module}/{table}` | `list` |
| PATCH | `/{module}/{table}/:id` | `update` |
| DELETE | `/{module}/{table}/:id` | `delete` |

示例：`POST /pms/devices`, `GET /pms/devices/:sn`

### 自动生成的代码

1. **SQL**: `CREATE TABLE devices (...)`
2. **Rust Model**: `pub struct Device { ... }`
3. **Rust Service**: `DeviceService { create, get, list, update, delete }`
4. **Rust API**: axum Router + handlers（含权限检查）
5. **TypeScript**: interface + client SDK
6. **React**: 列表 + 表单组件

---

## `#[api]` — 自定义端点

在 `#[db_resource]` 基础上添加自定义端点，或独立定义 API。

### 在 db_resource 基础上扩展

```rust
#[db_resource(module = "pms", table = "devices")]
pub struct Device { ... }

#[api(Device)]
#[handlers_path = "crate::handlers::device"]
impl DeviceApi {
    /// 配置设备
    #[endpoint(POST "/pms/devices/:sn/@provision")]
    #[permission("pms:device:provision")]
    #[handler = "provision"]
    async fn provision(sn: String, body: ProvisionRequest) -> Device;
    
    /// 激活设备
    #[endpoint(POST "/pms/devices/:sn/@activate")]
    #[permission("pms:device:activate")]
    #[handler = "activate"]
    async fn activate(sn: String) -> Device;
}
```

### 覆盖标准 CRUD

如果自定义端点的路径和方法与标准 CRUD 冲突，使用自定义实现：

```rust
#[api(Device)]
#[handlers_path = "crate::handlers::device"]
impl DeviceApi {
    /// 覆盖标准 create（需要额外的 SN 生成逻辑）
    #[endpoint(POST "/pms/devices")]
    #[permission("pms:device:create")]
    #[handler = "create"]
    async fn create(body: CreateDeviceRequest) -> Device;
}
```

### 独立 API（不关联 db_resource）

```rust
#[api]
impl HealthApi {
    #[endpoint(GET "/health")]
    #[public]  // 不需要认证
    async fn health() -> HealthResponse;
}
```

---

## 字段 Attributes

### 数据库

| Attribute | 说明 | 示例 |
|-----------|------|------|
| `#[primary_key]` | 主键 | `#[primary_key] pub id: String` |
| `#[unique]` | 唯一约束 | `#[unique] pub email: String` |
| `#[index]` | 创建索引 | `#[index] pub status: DeviceStatus` |
| `#[default(value)]` | 默认值 | `#[default(0)] pub retry_count: i64` |

### 自动填充

| Attribute | 说明 | 示例 |
|-----------|------|------|
| `#[auto_timestamp(on_create)]` | 创建时自动设置时间 | `pub created_at: String` |
| `#[auto_timestamp(on_update)]` | 更新时自动设置时间 | `pub updated_at: String` |
| `#[auto_user_id(on_create)]` | 创建时设置为当前用户 | `pub created_by: String` |

### UI 配置

```rust
#[ui(
    label = "显示标签",
    input_type = "text",       // 输入类型
    placeholder = "提示文字",
    help_text = "帮助说明",
    readonly = true,           // 只读
    hidden = true,             // 隐藏
)]
pub field_name: Type,
```

**支持的 input_type：**

| 类型 | 说明 | Rust 类型 |
|------|------|-----------|
| `text` | 单行文本 | `String` |
| `textarea` | 多行文本 | `String` |
| `number` | 数字 | `i32`, `i64`, `f64` |
| `email` | 邮箱 | `String` |
| `select` | 下拉选择 | `Enum` |
| `tags` | 标签输入 | `Vec<String>` |
| `json` | JSON 编辑器 | `serde_json::Value` |
| `password` | 密码 | `String` |
| `image_upload` | 图片上传 | `String` (URL) |
| `file_upload` | 文件上传 | `String` (URL) |

### 关系

| Attribute | 说明 | 示例 |
|-----------|------|------|
| `#[belongs_to(Target)]` | 多对一 | `#[belongs_to(Model)] pub model_id: String` |
| `#[has_many(Target, fk = "...")]` | 一对多 | `#[has_many(Device, fk = "batch_id")]` |

---

## 端点 Attributes

### `#[endpoint(METHOD "path")]`

定义 HTTP 端点。

```rust
#[endpoint(GET "/pms/devices/:sn")]
#[endpoint(POST "/pms/devices/:sn/@provision")]
#[endpoint(GET "/pms/devices")]
```

路径参数用 `:name` 表示。

### `#[permission("module:resource:action")]`

定义所需权限。生成的代码自动调用 `check_permission()`。

```rust
#[permission("pms:device:provision")]
```

### `#[public]`

标记为公开端点，不需要 JWT 认证。

```rust
#[endpoint(GET "/health")]
#[public]
async fn health() -> HealthResponse;
```

### `#[handler = "name"]`

映射到实现文件中的函数。

```rust
#[handler = "provision"]
// → 调用 crate::handlers::device::provision()
```

### `#[handlers_path = "path"]`

设置 impl 块中所有 handler 的基础路径。

```rust
#[api(Device)]
#[handlers_path = "crate::handlers::device"]
impl DeviceApi {
    #[handler = "provision"]
    // → crate::handlers::device::provision
    
    #[handler = "activate"]
    // → crate::handlers::device::activate
}
```

---

## 资源级 Attributes

### `#[permission(action = "permission")]`

定义 CRUD 操作的权限。

```rust
#[permission(create = "pms:device:create")]
#[permission(read = "pms:device:read")]
#[permission(update = "pms:device:update")]
#[permission(delete = "pms:device:delete")]
#[permission(list = "pms:device:list")]
```

如果某个操作不需要（如不允许删除），不定义该权限即可。

---

## 实现文件规范

### 函数签名

实现文件中的函数签名必须与 DSL 定义匹配：

```rust
// DSL 定义
#[endpoint(POST "/pms/devices/:sn/@provision")]
#[permission("pms:device:provision")]
#[handler = "provision"]
async fn provision(sn: String, body: ProvisionRequest) -> Device;
```

```rust
// 实现文件: rust/mod/pms/src/handlers/device/provision.rs
use super::*;

pub async fn provision(
    claims: Claims,                          // JWT claims（自动注入）
    State(svc): State<Arc<DeviceService>>,   // Service（自动注入）
    Path(sn): Path<String>,                  // 路径参数
    Json(body): Json<ProvisionRequest>,      // 请求体
) -> Result<Json<Device>, ApiError> {
    // 权限检查由生成的 wrapper 处理
    // 这里只写业务逻辑
    
    let mut device = svc.get(&sn).await?;
    
    if device.status != DeviceStatus::Pending {
        return Err(ApiError::bad_request("Device is not in Pending status"));
    }
    
    device.status = DeviceStatus::Provisioned;
    device.imei = body.imei_list;
    device.licenses = body.license_ids;
    
    let updated = svc.update(&sn, device).await?;
    Ok(Json(updated))
}
```

### 可以 import 其他库

实现文件就是普通的 Rust 代码，可以自由 import：

```rust
use crate::sn::SNGenerator;           // SN 生成器
use openerp_auth::PolicyService;       // 权限服务
use openerp_kv::KVStore;              // KV 存储
use tokio::time::Duration;            // 标准库
use anyhow::bail;                     // 错误处理
```

---

## 完整示例

### 简单资源（纯 CRUD）

```rust
// rust/lib/api/schema/auth/role.rs
use serde::{Deserialize, Serialize};

/// 角色定义，包含一组权限字符串。
#[db_resource(
    module = "auth",
    table = "roles",
    display_name = "Role",
)]
#[permission(create = "auth:role:create")]
#[permission(read = "auth:role:read")]
#[permission(update = "auth:role:update")]
#[permission(delete = "auth:role:delete")]
#[permission(list = "auth:role:list")]
pub struct Role {
    /// 唯一标识符（如 "pms:admin"）
    #[primary_key]
    #[ui(label = "Role ID", input_type = "text")]
    pub id: String,
    
    /// 描述
    #[ui(label = "Description", input_type = "textarea")]
    pub description: Option<String>,
    
    /// 权限列表
    #[ui(label = "Permissions", input_type = "tags")]
    pub permissions: Vec<String>,
    
    /// 注册服务
    #[ui(label = "Service", input_type = "text")]
    pub service: Option<String>,
    
    #[auto_timestamp(on_create)]
    pub created_at: String,
    
    #[auto_timestamp(on_update)]
    pub updated_at: String,
}
// 不需要 #[api] — 5 个 CRUD 端点自动生成
```

### 复杂资源（CRUD + 自定义）

```rust
// rust/lib/api/schema/task/task.rs
use serde::{Deserialize, Serialize};

#[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[db_resource(
    module = "task",
    table = "tasks",
    display_name = "Task",
)]
#[permission(create = "task:task:create")]
#[permission(read = "task:task:read")]
#[permission(list = "task:task:list")]
// 注意：没有 update 和 delete（任务不可直接修改/删除）
pub struct Task {
    #[primary_key]
    pub id: String,
    
    #[serde(rename = "type")]
    pub task_type: String,
    
    pub status: TaskStatus,
    // ...
}

#[api(Task)]
#[handlers_path = "crate::handlers::task"]
impl TaskApi {
    #[endpoint(POST "/task/tasks/:id/@claim")]
    #[permission("task:task:claim")]
    #[handler = "claim"]
    async fn claim(id: String, body: ClaimRequest) -> Task;
    
    #[endpoint(POST "/task/tasks/:id/@progress")]
    #[permission("task:task:progress")]
    #[handler = "progress"]
    async fn progress(id: String, body: ProgressReport) -> Task;
    
    #[endpoint(POST "/task/tasks/:id/@complete")]
    #[permission("task:task:complete")]
    #[handler = "complete"]
    async fn complete(id: String, body: CompleteRequest) -> Task;
    
    #[endpoint(POST "/task/tasks/:id/@fail")]
    #[permission("task:task:fail")]
    #[handler = "fail"]
    async fn fail(id: String, body: FailRequest) -> Task;
    
    #[endpoint(POST "/task/tasks/:id/@cancel")]
    #[permission("task:task:cancel")]
    #[handler = "cancel"]
    async fn cancel(id: String) -> Task;
    
    #[endpoint(GET "/task/tasks/:id/@poll")]
    #[permission("task:task:poll")]
    #[handler = "poll"]
    async fn poll(id: String, query: PollQuery) -> Task;
    
    #[endpoint(POST "/task/tasks/:id/@log")]
    #[permission("task:task:log")]
    #[handler = "log_write"]
    async fn log_write(id: String, body: LogRequest) -> ();
    
    #[endpoint(GET "/task/tasks/:id/@logs")]
    #[permission("task:task:read")]
    #[handler = "log_read"]
    async fn log_read(id: String, query: LogQuery) -> Vec<TaskLogEntry>;
}
```
