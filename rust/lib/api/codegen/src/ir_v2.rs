/// Enhanced Intermediate Representation (IR) V2
/// 
/// 支持全栈代码生成的完整元信息：
/// - 资源定义（字段、类型、约束）
/// - 关系定义（belongs_to, has_many, has_and_belongs_to_many）
/// - UI 配置（表单、列表、详情页）
/// - 数据库配置（索引、约束）
/// - 权限配置
/// - 自定义操作

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// 完整的 API schema（包含多个资源）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub resources: Vec<Resource>,
    pub enums: Vec<EnumDef>,
    pub structs: Vec<StructDef>,
}

/// 资源定义（对应一个数据库表 + CRUD API）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub fields: Vec<Field>,
    pub config: ResourceConfig,
    pub filters: Vec<Filter>,
    pub actions: Vec<Action>,
}

/// 资源配置（数据库、UI、权限）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    // 数据库配置
    pub table_name: String,
    pub indexes: Vec<Index>,
    
    // UI 配置
    pub display_name: String,
    pub display_name_plural: String,
    pub icon: Option<String>,
    pub list_columns: Vec<String>,
    pub searchable_fields: Vec<String>,
    pub default_sort: String,
    
    // 权限配置
    pub permissions: HashMap<String, Vec<String>>,  // action -> roles
}

/// 字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub attributes: FieldAttributes,
    pub ui: UIConfig,
}

/// 字段属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAttributes {
    pub is_primary_key: bool,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_indexed: bool,
    pub is_virtual: bool,         // 不存储在数据库
    pub is_computed: bool,        // 计算字段
    pub default_value: Option<String>,
    pub auto_timestamp: Option<TimestampType>,  // on_create / on_update
    pub auto_user_id: Option<TimestampType>,
    pub relation: Option<Relation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimestampType {
    OnCreate,
    OnUpdate,
}

/// 关系定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Relation {
    BelongsTo {
        target: String,
        foreign_key: Option<String>,
        optional: bool,
    },
    HasMany {
        target: String,
        foreign_key: String,
    },
    HasAndBelongsToMany {
        target: String,
        through: String,          // 中间表名
        join_table_config: Option<JoinTableConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinTableConfig {
    pub source_fk: String,
    pub target_fk: String,
}

/// UI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub label: Option<String>,
    pub input_type: InputType,
    pub placeholder: Option<String>,
    pub help_text: Option<String>,
    pub readonly: bool,
    pub hidden: bool,
    pub validation: Option<String>,  // JSON validation rules
    pub default: Option<String>,
    
    // Select/Multi-select 特定配置
    pub options_from: Option<OptionsSource>,
    pub fetch_options: Option<String>,  // API endpoint
    pub option_label: Option<String>,
    pub option_value: Option<String>,
    
    // 特定输入类型的配置
    pub rows: Option<i32>,              // textarea
    pub accept: Option<String>,         // file upload
    pub max_size: Option<String>,       // file upload
    pub foreign_key: bool,              // 是否是外键选择器
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    Text,
    Textarea,
    Number,
    Select,
    MultiSelect,
    Radio,
    Checkbox,
    Date,
    DateTime,
    Time,
    Email,
    Phone,
    Url,
    Color,
    ImageUpload,
    FileUpload,
    Markdown,
    Code,
    Tags,
    Object,         // 嵌套对象编辑器
    Array,          // 数组编辑器
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptionsSource {
    Enum,           // 从枚举类型生成
    Static(Vec<OptionItem>),
    Fetch(String),  // API endpoint
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionItem {
    pub label: String,
    pub value: String,
}

/// 类型系统
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Type {
    // 基础类型
    String,
    I32,
    I64,
    F64,
    Bool,
    DateTime,
    
    // 容器类型
    Option { inner: Box<Type> },
    Vec { inner: Box<Type> },
    HashMap { key: Box<Type>, value: Box<Type> },
    
    // 自定义类型
    Enum { name: String },
    Struct { name: String },
    
    // 特殊类型
    Json,           // 任意 JSON
    Binary,         // 二进制数据
}

/// 枚举定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub ui: EnumUIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<String>,  // 自定义序列化值
    pub ui_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumUIConfig {
    pub display_name: String,
}

/// 结构体定义（嵌套对象）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
}

/// 数据库索引
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Index {
    Single { column: String },
    Composite { columns: Vec<String> },
    Unique { columns: Vec<String> },
    FullText { columns: Vec<String> },
}

/// 查询过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub param_name: String,  // 查询参数名（如 name_like）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Eq,             // =
    Ne,             // !=
    Gt,             // >
    Gte,            // >=
    Lt,             // <
    Lte,            // <=
    In,             // IN (...)
    NotIn,          // NOT IN (...)
    Like,           // LIKE '%...%'
    StartsWith,     // LIKE '...%'
    EndsWith,       // LIKE '%...'
    IsNull,         // IS NULL
    IsNotNull,      // IS NOT NULL
    DateRange,      // BETWEEN ... AND ...
}

/// 自定义操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub resource: String,
    pub method: HttpMethod,
    pub path: String,
    pub params: Vec<ActionParam>,
    pub return_type: Type,
    pub config: ActionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionParam {
    pub name: String,
    pub ty: Type,
    pub source: ParamSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamSource {
    Path,
    Query,
    Body,
    Header,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    pub display_name: String,
    pub description: Option<String>,
    pub permission: Vec<String>,  // 需要的角色
    pub confirmation: Option<String>,  // 前端确认提示
}

// ========== 默认实现 ==========

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            label: None,
            input_type: InputType::Text,
            placeholder: None,
            help_text: None,
            readonly: false,
            hidden: false,
            validation: None,
            default: None,
            options_from: None,
            fetch_options: None,
            option_label: None,
            option_value: None,
            rows: None,
            accept: None,
            max_size: None,
            foreign_key: false,
        }
    }
}

impl Default for FieldAttributes {
    fn default() -> Self {
        Self {
            is_primary_key: false,
            is_required: false,
            is_unique: false,
            is_indexed: false,
            is_virtual: false,
            is_computed: false,
            default_value: None,
            auto_timestamp: None,
            auto_user_id: None,
            relation: None,
        }
    }
}
