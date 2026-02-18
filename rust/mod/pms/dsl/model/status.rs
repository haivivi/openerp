use openerp_macro::dsl_enum;

#[dsl_enum(module = "pms")]
pub enum BatchStatus {
    Draft,
    InProgress,
    Completed,
    Cancelled,
}

#[dsl_enum(module = "pms")]
pub enum DeviceStatus {
    Provisioned,
    Active,
    Inactive,
}

#[dsl_enum(module = "pms")]
pub enum FirmwareStatus {
    Uploaded,
    Released,
    Deprecated,
}

#[dsl_enum(module = "pms")]
pub enum LicenseStatus {
    Active,
    Expired,
    Revoked,
}
