/// Intermediate Representation (IR) - parsed API schema
#[derive(Debug, Clone)]
pub struct Schema {
    pub types: Vec<TypeDef>,
    pub service: Service,
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum Type {
    String,
    I32,
    I64,
    Bool,
    Option(Box<Type>),
    Vec(Box<Type>),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub method: HttpMethod,
    pub path: String,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub kind: ParamKind,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ParamKind {
    Path,
    Query,
    Body,
}
