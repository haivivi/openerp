// Character 资源 - 完整 DSL 示例
// 这是用 attribute macros 定义资源的示例，将自动生成：
// 1. SQL migration
// 2. Rust model + service
// 3. REST API handlers
// 4. TypeScript types + client
// 5. React 列表/表单/详情组件

use openerp_codegen::*;

// ========== 枚举定义 ==========

#[derive(Enum, Debug, Clone, Serialize, Deserialize)]
#[ui(display_name = "性别")]
pub enum Gender {
    #[ui(label = "男")]
    Male,
    #[ui(label = "女")]
    Female,
    #[ui(label = "其他")]
    Other,
}

#[derive(Enum, Debug, Clone, Serialize, Deserialize)]
#[ui(display_name = "角色类型")]
pub enum CharacterType {
    #[ui(label = "官方")]
    Official,
    #[ui(label = "社区")]
    Community,
    #[ui(label = "用户自定义")]
    Custom,
}

// ========== 嵌套结构 ==========

#[derive(Struct, Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Struct, Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMediaAssets {
    pub cover: Option<String>,
    pub gallery: Vec<String>,
}

// ========== 主资源定义 ==========

#[resource(
    // 数据库配置
    table = "characters",
    db_indexes = [
        Index::Single("type"),
        Index::Single("gender"),
        Index::Single("display_order"),
        Index::Composite(["type", "display_order"]),
    ],
    
    // UI 配置
    display_name = "角色",
    display_name_plural = "角色列表",
    icon = "user",
    
    // 列表页配置
    list_columns = ["name", "type", "gender", "age", "deviceNum", "displayOrder", "createAt"],
    searchable_fields = ["name", "intro", "keywords"],
    default_sort = "-displayOrder",
    
    // 权限配置
    permissions = [
        Permission::Create("admin", "editor"),
        Permission::Read("*"),  // 所有人可读
        Permission::Update("admin", "editor", "owner"),
        Permission::Delete("admin"),
    ],
)]
pub struct Character {
    // ========== 标准字段（自动添加，可选声明） ==========
    
    #[primary_key]
    #[ui(hidden = true)]  // 表单中隐藏
    pub id: String,
    
    #[auto_timestamp(on_create)]
    #[ui(readonly = true, label = "创建时间")]
    pub create_at: DateTime<Utc>,
    
    #[auto_timestamp(on_update)]
    #[ui(readonly = true, label = "更新时间")]
    pub update_at: DateTime<Utc>,
    
    #[auto_user_id(on_create)]
    #[ui(readonly = true, label = "创建者")]
    pub create_by: String,
    
    #[auto_user_id(on_update)]
    #[ui(readonly = true, label = "更新者")]
    pub update_by: String,
    
    // ========== 业务字段 ==========
    
    #[required]
    #[ui(
        label = "角色名称",
        placeholder = "请输入角色名称",
        input_type = "text",
        validation = r#"{ required: true, minLength: 2, maxLength: 50 }"#
    )]
    pub name: String,
    
    #[required]
    #[ui(
        label = "角色类型",
        input_type = "select",
        options_from = "enum"  // 从 CharacterType 枚举生成选项
    )]
    pub r#type: CharacterType,
    
    #[ui(
        label = "性别",
        input_type = "radio",
        options_from = "enum"
    )]
    pub gender: Option<Gender>,
    
    #[ui(
        label = "年龄",
        input_type = "number",
        validation = r#"{ min: 0, max: 200 }"#
    )]
    pub age: Option<i32>,
    
    #[ui(
        label = "角色介绍",
        input_type = "textarea",
        rows = 4,
        placeholder = "简要介绍角色特点"
    )]
    pub intro: Option<String>,
    
    #[ui(
        label = "背景故事",
        input_type = "markdown",  // 支持 markdown 编辑器
        rows = 10
    )]
    pub story: Option<String>,
    
    #[ui(
        label = "头像",
        input_type = "image_upload",
        accept = "image/*",
        max_size = "2MB"
    )]
    pub avatar: Option<String>,
    
    #[ui(
        label = "肖像图",
        input_type = "image_upload"
    )]
    pub portrait_photo_url: Option<String>,
    
    #[ui(
        label = "主题颜色",
        input_type = "color_picker",
        default = "#ffffff"
    )]
    pub bg_color: Option<String>,
    
    #[ui(
        label = "展示顺序",
        input_type = "number",
        default = 0,
        help_text = "数值越大越靠前"
    )]
    pub display_order: i32,
    
    #[ui(
        label = "适用系列",
        input_type = "multi_select",
        help_text = "选择角色适用的设备系列"
    )]
    pub series: Vec<i32>,  // 系列编号列表
    
    #[ui(
        label = "关键词",
        input_type = "tags",
        placeholder = "输入关键词后按回车"
    )]
    pub keywords: Option<String>,
    
    // ========== 嵌套对象 ==========
    
    #[ui(
        label = "设计师",
        input_type = "object",
        fields = ["name", "avatar"]
    )]
    pub designer: Option<Person>,
    
    #[ui(
        label = "动画师",
        input_type = "object"
    )]
    pub animator: Option<Person>,
    
    #[ui(
        label = "媒体资源",
        input_type = "object"
    )]
    pub media_assets: Option<CharacterMediaAssets>,
    
    // ========== 关联字段 ==========
    
    #[belongs_to(Voice, optional = true)]
    #[ui(
        label = "语音资源",
        input_type = "select",
        fetch_options = "/api/voices?_limit=100",
        option_label = "name",
        option_value = "id"
    )]
    pub voice_id: Option<String>,
    
    #[belongs_to(Voice, optional = true)]
    #[ui(label = "备用语音")]
    pub voice_backup_id: Option<String>,
    
    #[belongs_to(TunedLLM, optional = true)]
    #[ui(
        label = "预训练模型",
        input_type = "select",
        fetch_options = "/api/tuned-llms"
    )]
    pub tuned_llm_id: Option<String>,
    
    #[belongs_to(TunedLLM, optional = true)]
    #[ui(label = "备用模型")]
    pub tuned_llm_backup_id: Option<String>,
    
    #[has_many(Label, through = "character_labels")]
    #[ui(
        label = "标签",
        input_type = "multi_select",
        fetch_options = "/api/labels"
    )]
    pub label_ids: Vec<String>,
    
    // ========== 只读统计字段 ==========
    
    #[computed]
    #[ui(readonly = true, label = "使用设备数")]
    pub device_num: i32,
    
    #[computed]
    #[ui(readonly = true, label = "触发器总数")]
    pub trigger_count: i32,
    
    #[computed]
    #[ui(readonly = true, label = "小塑像总数")]
    pub figurine_count: i32,
    
    // ========== 虚拟字段（不存储在数据库） ==========
    
    #[virtual]
    #[ui(readonly = true)]
    pub age_string: Option<String>,  // 前端显示："18岁"
    
    #[virtual]
    #[ui(readonly = true)]
    pub gender_string: Option<String>,  // 前端显示："男"
}

// ========== 查询过滤器定义 ==========

#[filters(Character)]
pub struct CharacterFilters {
    // 精确匹配
    #[filter(Eq)]
    pub id: Option<String>,
    
    #[filter(In)]
    pub id_in: Option<Vec<String>>,
    
    #[filter(Eq)]
    pub r#type: Option<CharacterType>,
    
    #[filter(Eq)]
    pub gender: Option<Gender>,
    
    // 模糊搜索
    #[filter(Like)]
    pub name_like: Option<String>,
    
    #[filter(Like)]
    pub keywords_like: Option<String>,
    
    // 范围查询
    #[filter(Gte)]
    pub age_gte: Option<i32>,
    
    #[filter(Lte)]
    pub age_lte: Option<i32>,
    
    #[filter(Gte)]
    pub display_order_gte: Option<i32>,
    
    // 时间范围
    #[filter(DateRange)]
    pub create_at: Option<DateRange>,
    
    #[filter(DateRange)]
    pub update_at: Option<DateRange>,
    
    // 分页排序
    #[pagination]
    pub pagination: Pagination,  // { limit, offset, count }
    
    #[sorting(allowed = ["name", "displayOrder", "createAt", "updateAt"])]
    pub sort: Option<String>,  // "name" or "-name"
}

// ========== 自定义操作 ==========

#[action(
    resource = Character,
    method = POST,
    path = "/{id}/@clone",
    display_name = "克隆角色",
    description = "复制角色创建新角色",
    permission = "editor"
)]
pub struct CloneCharacterAction {
    #[path_param]
    pub id: String,
    
    #[body]
    pub new_name: String,
}

impl CloneCharacterAction {
    pub async fn handle(&self, service: &CharacterService) -> Result<Character> {
        let original = service.get(&self.id).await?;
        let mut cloned = original.clone();
        cloned.id = Uuid::new_v4().to_string();
        cloned.name = self.new_name.clone();
        service.create(cloned).await
    }
}

#[action(
    resource = Character,
    method = POST,
    path = "/@bulk_update_order",
    display_name = "批量更新排序",
    permission = "admin"
)]
pub struct BulkUpdateOrderAction {
    #[body]
    pub items: Vec<CharacterOrderItem>,
}

#[derive(Deserialize)]
pub struct CharacterOrderItem {
    pub id: String,
    pub display_order: i32,
}

impl BulkUpdateOrderAction {
    pub async fn handle(&self, service: &CharacterService) -> Result<()> {
        for item in &self.items {
            service.update_order(&item.id, item.display_order).await?;
        }
        Ok(())
    }
}
