use openerp_macro::dsl_enum;

#[dsl_enum(module = "task")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
