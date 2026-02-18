//! FlatBuffer code generation for `#[resource]` structs.
//!
//! Generates `IntoFlatBuffer`, `FromFlatBuffer`, `IntoFlatBufferList`,
//! `FromFlatBufferList` impls and a `.fbs` schema string const.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemStruct;

// ── Field classification ────────────────────────────────────────────

enum FbFieldKind {
    Scalar,
    String,
    OptScalar,
    OptString,
    ScalarVec,
    StringVec,
}

struct FbField {
    name: syn::Ident,
    kind: FbFieldKind,
    /// The scalar Rust type ident (e.g. `u32`, `bool`). None for String kinds.
    scalar_ty: Option<syn::Ident>,
    /// Field index (0-based), determines vtable offset via `4 + 2*index`.
    index: usize,
}

impl FbField {
    fn vt(&self) -> u16 {
        (4 + 2 * self.index) as u16
    }
}

/// Classify all named fields of a struct into FlatBuffer field kinds.
fn classify_fields(s: &ItemStruct) -> Vec<FbField> {
    let fields = match &s.fields {
        syn::Fields::Named(n) => &n.named,
        _ => return Vec::new(),
    };

    fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let name = f.ident.clone()?;
            let (kind, scalar_ty) = classify_type(&f.ty);
            Some(FbField {
                name,
                kind,
                scalar_ty,
                index: i,
            })
        })
        .collect()
}

fn classify_type(ty: &syn::Type) -> (FbFieldKind, Option<syn::Ident>) {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            match name.as_str() {
                "String" => return (FbFieldKind::String, None),
                "bool" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64"
                | "f32" | "f64" => {
                    return (FbFieldKind::Scalar, Some(seg.ident.clone()));
                }
                "Option" => {
                    if let Some(inner) = extract_generic_arg(&seg.arguments) {
                        let (inner_kind, inner_scalar) = classify_type(inner);
                        match inner_kind {
                            FbFieldKind::String => return (FbFieldKind::OptString, None),
                            FbFieldKind::Scalar => {
                                return (FbFieldKind::OptScalar, inner_scalar)
                            }
                            _ => {}
                        }
                    }
                }
                "Vec" => {
                    if let Some(inner) = extract_generic_arg(&seg.arguments) {
                        let (inner_kind, inner_scalar) = classify_type(inner);
                        match inner_kind {
                            FbFieldKind::String => return (FbFieldKind::StringVec, None),
                            FbFieldKind::Scalar => {
                                return (FbFieldKind::ScalarVec, inner_scalar)
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // Fallback: treat as String (will call .to_string() on Display types).
    (FbFieldKind::String, None)
}

fn extract_generic_arg(args: &syn::PathArguments) -> Option<&syn::Type> {
    if let syn::PathArguments::AngleBracketed(ab) = args {
        if let Some(syn::GenericArgument::Type(ty)) = ab.args.first() {
            return Some(ty);
        }
    }
    None
}

// ── Code generation ─────────────────────────────────────────────────

/// Generate all FlatBuffer trait impls for a resource struct.
pub fn emit_flatbuffer_impls(s: &ItemStruct) -> TokenStream {
    let fields = classify_fields(s);
    if fields.is_empty() {
        return quote! {};
    }

    let ident = &s.ident;
    let encode_table = emit_encode_table(ident, &fields);
    let decode_table = emit_decode_table(ident, &fields);
    let into_fb = emit_into_flatbuffer(ident);
    let from_fb = emit_from_flatbuffer(ident);
    let into_fb_list = emit_into_flatbuffer_list(ident);
    let from_fb_list = emit_from_flatbuffer_list(ident);
    let fbs_schema = emit_fbs_schema(ident, &fields);

    quote! {
        #encode_table
        #decode_table
        #into_fb
        #from_fb
        #into_fb_list
        #from_fb_list
        #fbs_schema
    }
}

/// Generate `__fb_encode_table` helper method.
fn emit_encode_table(ident: &syn::Ident, fields: &[FbField]) -> TokenStream {
    // Phase 1: create offsets for offset-type fields (strings, vectors).
    let offset_stmts: Vec<TokenStream> = fields
        .iter()
        .filter_map(|f| {
            let name = &f.name;
            let off_name = format_ident!("__fb_{}", name);
            match f.kind {
                FbFieldKind::String => Some(quote! {
                    let #off_name = __fb_builder.create_string(&self.#name);
                }),
                FbFieldKind::OptString => Some(quote! {
                    let #off_name = self.#name.as_ref().map(|s| __fb_builder.create_string(s));
                }),
                FbFieldKind::StringVec => Some(quote! {
                    let #off_name = openerp_types::create_string_vector(__fb_builder, &self.#name);
                }),
                FbFieldKind::ScalarVec => {
                    Some(quote! {
                        let #off_name = __fb_builder.create_vector(&self.#name);
                    })
                }
                _ => None,
            }
        })
        .collect();

    // Phase 2: push fields into table.
    let push_stmts: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let off_name = format_ident!("__fb_{}", name);
            let vt = f.vt();
            match f.kind {
                FbFieldKind::Scalar => {
                    let scalar_ty = f.scalar_ty.as_ref().unwrap();
                    quote! {
                        __fb_builder.push_slot_always::<#scalar_ty>(#vt, self.#name);
                    }
                }
                FbFieldKind::String | FbFieldKind::StringVec | FbFieldKind::ScalarVec => {
                    quote! {
                        __fb_builder.push_slot_always::<flatbuffers::WIPOffset<_>>(#vt, #off_name);
                    }
                }
                FbFieldKind::OptScalar => {
                    let scalar_ty = f.scalar_ty.as_ref().unwrap();
                    quote! {
                        if let Some(__fb_val) = self.#name {
                            __fb_builder.push_slot_always::<#scalar_ty>(#vt, __fb_val);
                        }
                    }
                }
                FbFieldKind::OptString => {
                    quote! {
                        if let Some(__fb_off) = #off_name {
                            __fb_builder.push_slot_always::<flatbuffers::WIPOffset<_>>(#vt, __fb_off);
                        }
                    }
                }
            }
        })
        .collect();

    quote! {
        impl #ident {
            #[doc(hidden)]
            pub fn __fb_encode_table<'__fb>(
                &self,
                __fb_builder: &mut flatbuffers::FlatBufferBuilder<'__fb>,
            ) -> flatbuffers::WIPOffset<flatbuffers::TableFinishedWIPOffset> {
                #(#offset_stmts)*
                let __fb_start = __fb_builder.start_table();
                #(#push_stmts)*
                __fb_builder.end_table(__fb_start)
            }

            #[doc(hidden)]
            pub fn __fb_decode_table(
                __fb_table: &flatbuffers::Table<'_>,
            ) -> Result<Self, openerp_types::FlatBufferDecodeError> {
                Self::__fb_decode_table_impl(__fb_table)
            }
        }
    }
}

/// Generate `__fb_decode_table_impl` static helper method.
fn emit_decode_table(ident: &syn::Ident, fields: &[FbField]) -> TokenStream {
    let field_reads: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let vt = f.vt();
            match f.kind {
                FbFieldKind::Scalar => {
                    let scalar_ty = f.scalar_ty.as_ref().unwrap();
                    let default = scalar_default(scalar_ty);
                    quote! {
                        let #name = unsafe {
                            __fb_table.get::<#scalar_ty>(#vt, Some(#default))
                        }.unwrap_or(#default);
                    }
                }
                FbFieldKind::String => {
                    quote! {
                        let #name = unsafe {
                            __fb_table.get::<flatbuffers::ForwardsUOffset<&str>>(#vt, None)
                        }.unwrap_or("").to_string();
                    }
                }
                FbFieldKind::OptScalar => {
                    let scalar_ty = f.scalar_ty.as_ref().unwrap();
                    quote! {
                        let #name: Option<#scalar_ty> = unsafe {
                            __fb_table.get::<#scalar_ty>(#vt, None)
                        };
                    }
                }
                FbFieldKind::OptString => {
                    quote! {
                        let #name: Option<String> = unsafe {
                            __fb_table.get::<flatbuffers::ForwardsUOffset<&str>>(#vt, None)
                        }.map(|s| s.to_string());
                    }
                }
                FbFieldKind::StringVec => {
                    quote! {
                        let #name: Vec<String> = unsafe {
                            __fb_table.get::<flatbuffers::ForwardsUOffset<
                                flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<&str>>
                            >>(#vt, None)
                        }.map(|v| v.iter().map(|s| s.to_string()).collect())
                         .unwrap_or_default();
                    }
                }
                FbFieldKind::ScalarVec => {
                    let scalar_ty = f.scalar_ty.as_ref().unwrap();
                    quote! {
                        let #name: Vec<#scalar_ty> = unsafe {
                            __fb_table.get::<flatbuffers::ForwardsUOffset<
                                flatbuffers::Vector<'_, #scalar_ty>
                            >>(#vt, None)
                        }.map(|v| v.iter().collect())
                         .unwrap_or_default();
                    }
                }
            }
        })
        .collect();

    let field_names: Vec<&syn::Ident> = fields.iter().map(|f| &f.name).collect();

    quote! {
        impl #ident {
            #[doc(hidden)]
            fn __fb_decode_table_impl(
                __fb_table: &flatbuffers::Table<'_>,
            ) -> Result<Self, openerp_types::FlatBufferDecodeError> {
                #(#field_reads)*
                Ok(Self { #(#field_names),* })
            }
        }
    }
}

/// Generate `IntoFlatBuffer` impl.
fn emit_into_flatbuffer(ident: &syn::Ident) -> TokenStream {
    quote! {
        impl openerp_types::IntoFlatBuffer for #ident {
            fn encode_flatbuffer(&self) -> Vec<u8> {
                let mut __fb_builder = flatbuffers::FlatBufferBuilder::with_capacity(256);
                let __fb_root = self.__fb_encode_table(&mut __fb_builder);
                __fb_builder.finish(__fb_root, None);
                __fb_builder.finished_data().to_vec()
            }
        }
    }
}

/// Generate `FromFlatBuffer` impl.
fn emit_from_flatbuffer(ident: &syn::Ident) -> TokenStream {
    quote! {
        impl openerp_types::FromFlatBuffer for #ident {
            fn decode_flatbuffer(
                buf: &[u8],
            ) -> Result<Self, openerp_types::FlatBufferDecodeError> {
                if buf.len() < 4 {
                    return Err(openerp_types::FlatBufferDecodeError::new("buffer too small"));
                }
                // SAFETY: We produced this buffer via FlatBufferBuilder, so the
                // root offset is valid. root_unchecked skips verification for
                // performance — Table does not implement Verifiable in all
                // flatbuffers crate versions.
                let __fb_table = unsafe {
                    flatbuffers::root_unchecked::<flatbuffers::Table>(buf)
                };
                Self::__fb_decode_table(&__fb_table)
            }
        }
    }
}

/// Generate `IntoFlatBufferList` impl.
fn emit_into_flatbuffer_list(ident: &syn::Ident) -> TokenStream {
    quote! {
        impl openerp_types::IntoFlatBufferList for #ident {
            fn encode_flatbuffer_list(items: &[Self], has_more: bool) -> Vec<u8> {
                let mut __fb_builder = flatbuffers::FlatBufferBuilder::with_capacity(
                    items.len() * 128 + 64,
                );
                let __fb_offsets: Vec<flatbuffers::WIPOffset<flatbuffers::TableFinishedWIPOffset>> =
                    items
                        .iter()
                        .map(|item| item.__fb_encode_table(&mut __fb_builder))
                        .collect();
                let __fb_items_vec = __fb_builder.create_vector(&__fb_offsets);

                let __fb_start = __fb_builder.start_table();
                __fb_builder.push_slot_always::<flatbuffers::WIPOffset<_>>(4, __fb_items_vec);
                __fb_builder.push_slot_always::<bool>(6, has_more);
                let __fb_root = __fb_builder.end_table(__fb_start);
                __fb_builder.finish(__fb_root, None);
                __fb_builder.finished_data().to_vec()
            }
        }
    }
}

/// Generate `FromFlatBufferList` impl.
fn emit_from_flatbuffer_list(ident: &syn::Ident) -> TokenStream {
    quote! {
        impl openerp_types::FromFlatBufferList for #ident {
            fn decode_flatbuffer_list(
                buf: &[u8],
            ) -> Result<(Vec<Self>, bool), openerp_types::FlatBufferDecodeError> {
                if buf.len() < 4 {
                    return Err(openerp_types::FlatBufferDecodeError::new("buffer too small"));
                }
                let __fb_list = unsafe {
                    flatbuffers::root_unchecked::<flatbuffers::Table>(buf)
                };
                let has_more = unsafe {
                    __fb_list.get::<bool>(6, Some(false))
                }.unwrap_or(false);

                let items = unsafe {
                    __fb_list.get::<flatbuffers::ForwardsUOffset<
                        flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<flatbuffers::Table<'_>>>
                    >>(4, None)
                }.map(|vec| {
                    vec.iter()
                        .map(|t| Self::__fb_decode_table(&t))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();

                Ok((items, has_more))
            }
        }
    }
}

/// Generate `.fbs` schema string constant.
fn emit_fbs_schema(ident: &syn::Ident, fields: &[FbField]) -> TokenStream {
    let table_name = ident.to_string();
    let mut schema = format!("table {} {{\n", table_name);
    for f in fields {
        let field_name = crate::util::to_snake_case(&f.name.to_string());
        let fbs_type = fbs_type_name(f);
        schema.push_str(&format!("    {}: {};\n", field_name, fbs_type));
    }
    schema.push_str("}\n");

    let list_table_name = format!("{}List", table_name);
    schema.push_str(&format!(
        "\ntable {} {{\n    items: [{}];\n    has_more: bool;\n}}\n",
        list_table_name, table_name
    ));

    let const_name = format_ident!("__FBS_SCHEMA_{}", table_name.to_uppercase());

    quote! {
        #[doc(hidden)]
        pub const #const_name: &str = #schema;
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Get the default value token for a scalar type.
fn scalar_default(ty: &syn::Ident) -> TokenStream {
    let name = ty.to_string();
    match name.as_str() {
        "bool" => quote! { false },
        "f32" => quote! { 0.0f32 },
        "f64" => quote! { 0.0f64 },
        _ => quote! { 0 },
    }
}

/// Map a field to its FlatBuffer schema type name.
fn fbs_type_name(f: &FbField) -> String {
    match f.kind {
        FbFieldKind::Scalar | FbFieldKind::OptScalar => {
            rust_to_fbs_scalar(f.scalar_ty.as_ref().unwrap())
        }
        FbFieldKind::String | FbFieldKind::OptString => "string".to_string(),
        FbFieldKind::StringVec => "[string]".to_string(),
        FbFieldKind::ScalarVec => {
            format!("[{}]", rust_to_fbs_scalar(f.scalar_ty.as_ref().unwrap()))
        }
    }
}

fn rust_to_fbs_scalar(ty: &syn::Ident) -> String {
    match ty.to_string().as_str() {
        "bool" => "bool",
        "u8" => "ubyte",
        "u16" => "ushort",
        "u32" => "uint",
        "u64" => "ulong",
        "i8" => "byte",
        "i16" => "short",
        "i32" => "int",
        "i64" => "long",
        "f32" => "float",
        "f64" => "double",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_types_classified() {
        let ty: syn::Type = syn::parse_str("u32").unwrap();
        let (kind, scalar) = classify_type(&ty);
        assert!(matches!(kind, FbFieldKind::Scalar));
        assert_eq!(scalar.unwrap().to_string(), "u32");
    }

    #[test]
    fn string_classified() {
        let ty: syn::Type = syn::parse_str("String").unwrap();
        let (kind, scalar) = classify_type(&ty);
        assert!(matches!(kind, FbFieldKind::String));
        assert!(scalar.is_none());
    }

    #[test]
    fn option_string_classified() {
        let ty: syn::Type = syn::parse_str("Option<String>").unwrap();
        let (kind, _) = classify_type(&ty);
        assert!(matches!(kind, FbFieldKind::OptString));
    }

    #[test]
    fn option_scalar_classified() {
        let ty: syn::Type = syn::parse_str("Option<u64>").unwrap();
        let (kind, scalar) = classify_type(&ty);
        assert!(matches!(kind, FbFieldKind::OptScalar));
        assert_eq!(scalar.unwrap().to_string(), "u64");
    }

    #[test]
    fn vec_string_classified() {
        let ty: syn::Type = syn::parse_str("Vec<String>").unwrap();
        let (kind, _) = classify_type(&ty);
        assert!(matches!(kind, FbFieldKind::StringVec));
    }

    #[test]
    fn vec_scalar_classified() {
        let ty: syn::Type = syn::parse_str("Vec<i32>").unwrap();
        let (kind, scalar) = classify_type(&ty);
        assert!(matches!(kind, FbFieldKind::ScalarVec));
        assert_eq!(scalar.unwrap().to_string(), "i32");
    }

    #[test]
    fn fbs_scalar_mapping() {
        let ident = syn::Ident::new("u32", proc_macro2::Span::call_site());
        assert_eq!(rust_to_fbs_scalar(&ident), "uint");
        let ident = syn::Ident::new("bool", proc_macro2::Span::call_site());
        assert_eq!(rust_to_fbs_scalar(&ident), "bool");
        let ident = syn::Ident::new("f64", proc_macro2::Span::call_site());
        assert_eq!(rust_to_fbs_scalar(&ident), "double");
    }
}
