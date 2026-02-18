//! Format negotiation and multi-format response wrapper.
//!
//! Provides `negotiate_format()` to read the `Accept` header and choose
//! between JSON and FlatBuffers. `FacetResponse<T>` is an axum
//! `IntoResponse` that serializes the value in the negotiated format.
//!
//! Usage in a hand-written facet handler:
//! ```ignore
//! async fn get_device(
//!     headers: HeaderMap,
//!     State(s): State<AppState>,
//! ) -> FacetResponse<AppDevice> {
//!     let device = load_device(&s).await?;
//!     FacetResponse::negotiate(device, &headers)
//! }
//! ```

use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use openerp_types::{Format, IntoFlatBuffer, MIME_FLATBUFFERS, MIME_JSON};
use serde::Serialize;

/// Read the `Accept` header and return the preferred response format.
///
/// If the header contains `application/x-flatbuffers`, returns `FlatBuffers`.
/// Otherwise defaults to `Json`.
pub fn negotiate_format(headers: &HeaderMap) -> Format {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(Format::from_accept)
        .unwrap_or(Format::Json)
}

/// A response that serializes as JSON or FlatBuffers based on content negotiation.
///
/// `T` must implement both `Serialize` (for JSON) and `IntoFlatBuffer`
/// (for FlatBuffers). The `#[facet]` macro generates both impls for
/// `#[resource]` structs.
pub struct FacetResponse<T> {
    value: T,
    format: Format,
}

impl<T> FacetResponse<T> {
    /// Create a response with an explicit format.
    pub fn new(value: T, format: Format) -> Self {
        Self { value, format }
    }

    /// Create a response by negotiating format from the Accept header.
    pub fn negotiate(value: T, headers: &HeaderMap) -> Self {
        Self {
            value,
            format: negotiate_format(headers),
        }
    }
}

impl<T> IntoResponse for FacetResponse<T>
where
    T: Serialize + IntoFlatBuffer,
{
    fn into_response(self) -> Response {
        match self.format {
            Format::Json => {
                let body = match serde_json::to_vec(&self.value) {
                    Ok(b) => b,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("json serialize: {}", e),
                        )
                            .into_response();
                    }
                };
                (
                    [(header::CONTENT_TYPE, MIME_JSON)],
                    body,
                )
                    .into_response()
            }
            Format::FlatBuffers => {
                let body = self.value.encode_flatbuffer();
                (
                    [(header::CONTENT_TYPE, MIME_FLATBUFFERS)],
                    body,
                )
                    .into_response()
            }
        }
    }
}

/// A list response that serializes as JSON or FlatBuffers.
///
/// For JSON: `{ "items": [...], "hasMore": bool }`
/// For FlatBuffers: root table with items vector + has_more field.
pub struct FacetListResponse<T> {
    items: Vec<T>,
    has_more: bool,
    format: Format,
}

impl<T> FacetListResponse<T> {
    pub fn new(items: Vec<T>, has_more: bool, format: Format) -> Self {
        Self {
            items,
            has_more,
            format,
        }
    }

    pub fn negotiate(items: Vec<T>, has_more: bool, headers: &HeaderMap) -> Self {
        Self {
            items,
            has_more,
            format: negotiate_format(headers),
        }
    }
}

impl<T> IntoResponse for FacetListResponse<T>
where
    T: Serialize + openerp_types::IntoFlatBufferList,
{
    fn into_response(self) -> Response {
        match self.format {
            Format::Json => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct ListBody<'a, T: Serialize> {
                    items: &'a [T],
                    has_more: bool,
                }
                let body = ListBody {
                    items: &self.items,
                    has_more: self.has_more,
                };
                let bytes = match serde_json::to_vec(&body) {
                    Ok(b) => b,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("json serialize: {}", e),
                        )
                            .into_response();
                    }
                };
                ([(header::CONTENT_TYPE, MIME_JSON)], bytes).into_response()
            }
            Format::FlatBuffers => {
                let body = T::encode_flatbuffer_list(&self.items, self.has_more);
                ([(header::CONTENT_TYPE, MIME_FLATBUFFERS)], body).into_response()
            }
        }
    }
}
