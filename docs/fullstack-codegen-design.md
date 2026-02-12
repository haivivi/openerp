# 全栈代码生成器设计文档

> 从 Rust DSL 一次性生成后端 + 前端 + 数据库代码

## 目标

为 haivivi-apps PAL 系统（30+ 资源，323 个 API 端点）设计一个全栈代码生成器，自动生成：

1. **后端**：Rust model + service + REST API + SQL schema
2. **前端**：TypeScript types + React 组件（列表/表单/详情）
3. **客户端**：TypeScript client SDK

## PAL 系统特征分析

### 典型资源模式

从 PAL 的 23,300 行 OpenAPI 定义中提取的模式：

#### 1. 标准字段（所有资源共享）

```typescript
interface BaseEntity {
  id: string;              // UUID 主键
  createAt: DateTime;      // 创建时间
  updateAt: DateTime;      // 更新时间
  createBy: string;        // 创建者 UID
  updateBy: string;        // 更新者 UID
}
```

#### 2. 资源关系

```
Payment (1) → (*) Order
Payment (1) → (*) Subscription
DeviceGenRecord (*) → (1) DeviceSeries
DeviceGenRecord (*) → (1) DeviceSeller
DeviceGenRecord (*) → (1) DeviceMfr
MiitPool (1) → (*) MiitLicense
```

#### 3. 枚举类型

```rust
enum DeviceGenRecordStatus {
    UNSPECIFIED,
    PENDING,
    PROVISIONED,
    ACTIVATED,
    RETIRED,
}

enum FirmwareStatus {
    DRAFT,
    PUBLISHED,
    DEPRECATED,
}
```

#### 4. 复杂查询

每个资源的 list 端点支持：

```
GET /device-gen-records?
  id=xxx,yyy              # 按 ID 列表筛选
  &type=PRODUCTION        # 按枚举筛选
  &seriesId=aaa,bbb       # 按外键筛选
  &description_like=test  # 模糊搜索
  &createAt_gte=2024-01-01  # 时间范围
  &createAt_lt=2024-12-31
  &_sort=-createAt        # 排序（- 表示降序）
  &_limit=20              # 分页大小
  &_offset=40             # 分页偏移
  &_count=true            # 是否返回总数
```

#### 5. 自定义操作

```
POST /device-gen-records/{id}/@provision  # 配置设备
POST /miit-licenses/@import              # 批量导入
POST /firmwares/@upload                  # 上传文件
```

#### 6. 前端需求

- **列表页**：表格（可排序、可搜索、分页）
- **表单页**：创建/编辑表单（字段验证、关联选择器）
- **详情页**：只读展示 + 关联资源

---

## DSL 设计

### 核心思路

**使用 Rust 作为 DSL 宿主语言**，通过 attribute macros 添加元信息：

```rust
#[resource(
    table = "device_gen_records",
    display_name = "设备生成记录",
    list_columns = ["sn", "eid", "type", "status", "createAt"],
    searchable = ["sn", "eid", "description"],
    default_sort = "-createAt",
)]
pub struct DeviceGenRecord {
    #[primary_key]
    pub id: String,
    
    #[unique]
    #[index]
    #[ui(label = "序列号", input_type = "text", required = true)]
    pub sn: String,
    
    #[unique]
    #[ui(label = "EID", input_type = "text", required = true)]
    pub eid: String,
    
    #[ui(label = "类型", input_type = "select")]
    pub r#type: DeviceGenRecordType,
    
    #[default(DeviceGenRecordStatus::PENDING)]
    #[ui(label = "状态", input_type = "select")]
    pub status: DeviceGenRecordStatus,
    
    #[belongs_to(DeviceSeries)]
    #[index]
    #[ui(label = "系列", input_type = "select", foreign_key = true)]
    pub series_id: String,
    
    #[belongs_to(DeviceSeller)]
    #[index]
    pub seller_id: String,
    
    #[belongs_to(DeviceMfr)]
    #[index]
    pub mfr_id: String,
    
    #[ui(label = "SKU", input_type = "text")]
    pub sku: Option<String>,
    
    #[ui(label = "描述", input_type = "textarea")]
    pub description: Option<String>,
    
    #[ui(label = "IMEI 列表", input_type = "tags")]
    pub imei: Vec<String>,
    
    // 标准字段（自动添加，可省略）
    // pub create_at: DateTime,
    // pub update_at: DateTime,
    // pub create_by: String,
    // pub update_by: String,
}

#[derive(Enum)]
#[ui(display_name = "设备生成记录类型")]
pub enum DeviceGenRecordType {
    #[ui(label = "生产")]
    PRODUCTION,
    #[ui(label = "测试")]
    TEST,
    #[ui(label = "样品")]
    SAMPLE,
}

#[derive(Enum)]
pub enum DeviceGenRecordStatus {
    UNSPECIFIED,
    PENDING,
    PROVISIONED,
    ACTIVATED,
    RETIRED,
}

// 定义自定义操作
#[resource_action(
    method = POST,
    path = "/{id}/@provision",
    handler = "provision_device",
)]
pub struct ProvisionDeviceAction {
    pub device_id: String,
    pub imei_list: Vec<String>,
    pub license_pool_id: String,
}
```

### 关系定义

```rust
// 一对多
#[resource]
pub struct DeviceSeries {
    #[primary_key]
    pub id: String,
    
    #[has_many(DeviceGenRecord, foreign_key = "series_id")]
    pub devices: Relation<DeviceGenRecord>,
}

// 多对多（通过中间表）
#[resource]
pub struct User {
    #[primary_key]
    pub id: String,
    
    #[has_and_belongs_to_many(Role, through = "user_roles")]
    pub roles: Relation<Role>,
}
```

### 查询过滤器

```rust
#[resource(
    filters = [
        // 自动生成的基础过滤器
        Filter::Eq("id"),
        Filter::In("id"),
        Filter::Like("description"),
        
        // 时间范围过滤器（自动生成）
        Filter::DateRange("createAt"),
        Filter::DateRange("updateAt"),
        
        // 枚举过滤器
        Filter::Eq("type"),
        Filter::Eq("status"),
        
        // 外键过滤器
        Filter::In("series_id"),
        Filter::In("seller_id"),
    ]
)]
pub struct DeviceGenRecord { /* ... */ }
```

---

## 生成内容详解

### 1. 数据库 Schema (SQL Migration)

```sql
-- 自动生成
CREATE TABLE device_gen_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sn VARCHAR(255) NOT NULL UNIQUE,
    eid VARCHAR(255) NOT NULL UNIQUE,
    type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'PENDING',
    series_id UUID NOT NULL REFERENCES device_series(id),
    seller_id UUID NOT NULL REFERENCES device_sellers(id),
    mfr_id UUID NOT NULL REFERENCES device_mfrs(id),
    sku VARCHAR(255),
    description TEXT,
    imei TEXT[],  -- PostgreSQL array
    create_at TIMESTAMP NOT NULL DEFAULT NOW(),
    update_at TIMESTAMP NOT NULL DEFAULT NOW(),
    create_by UUID NOT NULL,
    update_by UUID NOT NULL
);

CREATE INDEX idx_device_gen_records_sn ON device_gen_records(sn);
CREATE INDEX idx_device_gen_records_eid ON device_gen_records(eid);
CREATE INDEX idx_device_gen_records_series_id ON device_gen_records(series_id);
CREATE INDEX idx_device_gen_records_status ON device_gen_records(status);
```

### 2. Rust Model

```rust
// 自动生成
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGenRecord {
    pub id: String,
    pub sn: String,
    pub eid: String,
    pub r#type: DeviceGenRecordType,
    pub status: DeviceGenRecordStatus,
    pub series_id: String,
    pub seller_id: String,
    pub mfr_id: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub imei: Vec<String>,
    pub create_at: DateTime<Utc>,
    pub update_at: DateTime<Utc>,
    pub create_by: String,
    pub update_by: String,
    
    // 关联查询结果（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<Box<DeviceSeries>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller: Option<Box<DeviceSeller>>,
}
```

### 3. Service 层 (CRUD + 查询)

```rust
// 自动生成
pub struct DeviceGenRecordService {
    db: Arc<dyn SQLStore>,
}

impl DeviceGenRecordService {
    pub async fn create(&self, req: CreateDeviceGenRecordRequest) -> Result<DeviceGenRecord>;
    pub async fn get(&self, id: &str) -> Result<DeviceGenRecord>;
    pub async fn list(&self, params: ListDeviceGenRecordParams) -> Result<Page<DeviceGenRecord>>;
    pub async fn update(&self, id: &str, req: UpdateDeviceGenRecordRequest) -> Result<DeviceGenRecord>;
    pub async fn delete(&self, id: &str) -> Result<()>;
    
    // 自定义操作
    pub async fn provision(&self, id: &str, req: ProvisionDeviceRequest) -> Result<DeviceGenRecord>;
}

#[derive(Debug, Deserialize)]
pub struct ListDeviceGenRecordParams {
    pub id: Option<Vec<String>>,
    pub r#type: Option<DeviceGenRecordType>,
    pub status: Option<DeviceGenRecordStatus>,
    pub series_id: Option<Vec<String>>,
    pub description_like: Option<String>,
    pub create_at_gte: Option<DateTime<Utc>>,
    pub create_at_lt: Option<DateTime<Utc>>,
    pub _sort: Option<String>,  // "createAt" or "-createAt"
    pub _limit: Option<u32>,
    pub _offset: Option<u32>,
    pub _count: Option<bool>,
}
```

### 4. REST API Handlers

```rust
// 自动生成
pub fn device_gen_record_routes() -> Router {
    Router::new()
        .route("/device-gen-records", post(create_handler).get(list_handler))
        .route("/device-gen-records/:id", get(get_handler).patch(update_handler).delete(delete_handler))
        .route("/device-gen-records/:id/@provision", post(provision_handler))
}

async fn list_handler(
    Query(params): Query<ListDeviceGenRecordParams>,
    State(svc): State<Arc<DeviceGenRecordService>>,
) -> Result<Json<Vec<DeviceGenRecord>>, ApiError> {
    let result = svc.list(params).await?;
    Ok(Json(result.items))
}
```

### 5. TypeScript Types

```typescript
// 自动生成
export interface DeviceGenRecord {
  id: string;
  sn: string;
  eid: string;
  type: DeviceGenRecordType;
  status: DeviceGenRecordStatus;
  seriesId: string;
  sellerId: string;
  mfrId: string;
  sku?: string;
  description?: string;
  imei: string[];
  createAt: string;  // ISO 8601
  updateAt: string;
  createBy: string;
  updateBy: string;
  
  // 关联数据
  series?: DeviceSeries;
  seller?: DeviceSeller;
}

export enum DeviceGenRecordType {
  PRODUCTION = 'PRODUCTION',
  TEST = 'TEST',
  SAMPLE = 'SAMPLE',
}

export enum DeviceGenRecordStatus {
  UNSPECIFIED = 'UNSPECIFIED',
  PENDING = 'PENDING',
  PROVISIONED = 'PROVISIONED',
  ACTIVATED = 'ACTIVATED',
  RETIRED = 'RETIRED',
}

export interface ListDeviceGenRecordParams {
  id?: string[];
  type?: DeviceGenRecordType;
  status?: DeviceGenRecordStatus;
  seriesId?: string[];
  description_like?: string;
  createAt_gte?: string;
  createAt_lt?: string;
  _sort?: string;
  _limit?: number;
  _offset?: number;
  _count?: boolean;
}
```

### 6. TypeScript Client SDK

```typescript
// 自动生成
export class DeviceGenRecordClient {
  constructor(private baseUrl: string, private auth: AuthProvider) {}
  
  async create(req: CreateDeviceGenRecordRequest): Promise<DeviceGenRecord> {
    const response = await fetch(`${this.baseUrl}/device-gen-records`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${await this.auth.getToken()}`,
      },
      body: JSON.stringify(req),
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  }
  
  async list(params: ListDeviceGenRecordParams): Promise<DeviceGenRecord[]> {
    const query = new URLSearchParams();
    // ... 构建 query string
    const response = await fetch(`${this.baseUrl}/device-gen-records?${query}`);
    return response.json();
  }
  
  async provision(id: string, req: ProvisionDeviceRequest): Promise<DeviceGenRecord> {
    // ...
  }
}
```

### 7. React 列表组件

```tsx
// 自动生成
export function DeviceGenRecordList() {
  const [records, setRecords] = useState<DeviceGenRecord[]>([]);
  const [params, setParams] = useState<ListDeviceGenRecordParams>({
    _limit: 20,
    _offset: 0,
    _sort: '-createAt',
  });
  
  const columns = [
    { key: 'sn', label: '序列号', sortable: true },
    { key: 'eid', label: 'EID', sortable: true },
    { key: 'type', label: '类型', render: (v) => DeviceGenRecordTypeLabel[v] },
    { key: 'status', label: '状态', render: (v) => <StatusBadge status={v} /> },
    { key: 'createAt', label: '创建时间', sortable: true, render: formatDate },
  ];
  
  return (
    <div className="device-gen-record-list">
      <DeviceGenRecordFilters params={params} onChange={setParams} />
      <DataTable columns={columns} data={records} />
      <Pagination {...paginationProps} />
    </div>
  );
}
```

### 8. React 表单组件

```tsx
// 自动生成
export function DeviceGenRecordForm({ id }: { id?: string }) {
  const form = useForm<CreateDeviceGenRecordRequest>();
  
  return (
    <Form onSubmit={form.handleSubmit(onSubmit)}>
      <FormField
        label="序列号"
        name="sn"
        type="text"
        required
        {...form.register('sn', { required: true })}
      />
      <FormField
        label="EID"
        name="eid"
        type="text"
        required
        {...form.register('eid', { required: true })}
      />
      <FormField
        label="类型"
        name="type"
        type="select"
        options={DeviceGenRecordTypeOptions}
        {...form.register('type')}
      />
      <FormField
        label="系列"
        name="seriesId"
        type="select"
        options={deviceSeriesOptions}  // 从 API 加载
        required
        {...form.register('seriesId', { required: true })}
      />
      <FormField
        label="描述"
        name="description"
        type="textarea"
        {...form.register('description')}
      />
      <Button type="submit">保存</Button>
    </Form>
  );
}
```

---

## 实施计划

### Week 1: 核心基础设施

1. **Day 1-2**: DSL 设计 + IR 重构
   - 定义完整的 attribute macro 语法
   - 扩展 IR 结构（关系、约束、元数据）
   - 编写 3 个典型资源的 DSL 示例

2. **Day 3**: Proc macro 实现
   - 解析 Rust struct + attributes → IR
   - 生成包含 IR 的 codegen binary

### Week 2: 后端完整生成

3. **Day 4**: 数据库 schema 生成
   - SQL migration (CREATE TABLE + INDEX)
   - 支持关系外键

4. **Day 5**: Rust model + service 生成
   - Model struct with serde
   - Service CRUD methods
   - 关联查询支持

5. **Day 6**: REST API 生成
   - Handlers with extractors
   - 过滤、排序、分页
   - 自定义操作

### Week 3: 前端完整生成

6. **Day 7-8**: TypeScript 生成
   - Types + Enums
   - Client SDK

7. **Day 9-10**: React 组件生成
   - 列表组件
   - 表单组件
   - 详情页组件

### Week 4: 验证和优化

8. **Day 11-12**: 端到端验证
   - Device 资源完整生成
   - 手动测试前后端
   - 修复 bug

---

## 下一步

立即开始实施 Phase 1.1：
1. 从 PAL 提取 3 个典型资源的完整定义
2. 设计 attribute macro 语法
3. 编写完整的 DSL 示例

**开始时间**：现在  
**预计完成**：2-3 小时
