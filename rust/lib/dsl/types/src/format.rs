//! Multi-format response support.
//!
//! Defines the `Format` enum for content negotiation and traits for
//! FlatBuffer encoding/decoding. Used by `#[facet]` macro-generated code
//! and the facet client.

use flatbuffers::FlatBufferBuilder;

// ── Format enum ─────────────────────────────────────────────────────

/// Wire format for facet API responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// JSON — `application/json`. Default, human-readable.
    Json,
    /// FlatBuffers — `application/x-flatbuffers`. Zero-copy binary.
    FlatBuffers,
}

impl Format {
    /// MIME type string for this format.
    pub fn mime(&self) -> &'static str {
        match self {
            Format::Json => MIME_JSON,
            Format::FlatBuffers => MIME_FLATBUFFERS,
        }
    }

    /// Parse from an Accept header value.
    ///
    /// Returns `FlatBuffers` if the header contains `application/x-flatbuffers`,
    /// otherwise defaults to `Json`.
    pub fn from_accept(accept: &str) -> Self {
        if accept.contains(MIME_FLATBUFFERS) {
            Format::FlatBuffers
        } else {
            Format::Json
        }
    }

    /// Parse from a Content-Type header value.
    pub fn from_content_type(ct: &str) -> Self {
        if ct.contains(MIME_FLATBUFFERS) {
            Format::FlatBuffers
        } else {
            Format::Json
        }
    }
}

impl Default for Format {
    fn default() -> Self {
        Format::Json
    }
}

/// MIME type for JSON responses.
pub const MIME_JSON: &str = "application/json";

/// MIME type for FlatBuffers responses.
pub const MIME_FLATBUFFERS: &str = "application/x-flatbuffers";

// ── FlatBuffer traits ───────────────────────────────────────────────

/// Encode a value into a FlatBuffer byte vector.
///
/// Implemented by `#[facet]` macro for each `#[resource]` struct.
/// The output is a finished FlatBuffer that can be sent over the wire
/// and decoded zero-copy on the client side.
pub trait IntoFlatBuffer {
    /// Encode this value into a finished FlatBuffer.
    fn encode_flatbuffer(&self) -> Vec<u8>;
}

/// Encode a list of values into a single FlatBuffer.
///
/// Used for list endpoints that return `{ items: [...], hasMore: bool }`.
/// The generated code wraps items in a root table with a vector field.
pub trait IntoFlatBufferList: IntoFlatBuffer {
    /// Encode a list response (items + has_more) into a FlatBuffer.
    fn encode_flatbuffer_list(items: &[Self], has_more: bool) -> Vec<u8>
    where
        Self: Sized;
}

/// Decode a value from FlatBuffer bytes.
///
/// Implemented by `#[facet]` macro for each `#[resource]` struct.
/// Used by the client to decode responses when `Format::FlatBuffers` is selected.
pub trait FromFlatBuffer: Sized {
    /// Decode from a finished FlatBuffer byte slice.
    fn decode_flatbuffer(buf: &[u8]) -> Result<Self, FlatBufferDecodeError>;
}

/// Decode a list response from FlatBuffer bytes.
pub trait FromFlatBufferList: FromFlatBuffer {
    /// Decode a list response, returning (items, has_more).
    fn decode_flatbuffer_list(buf: &[u8]) -> Result<(Vec<Self>, bool), FlatBufferDecodeError>
    where
        Self: Sized;
}

/// Error when decoding a FlatBuffer fails.
#[derive(Debug, Clone)]
pub struct FlatBufferDecodeError {
    pub message: String,
}

impl FlatBufferDecodeError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for FlatBufferDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "flatbuffer decode: {}", self.message)
    }
}

impl std::error::Error for FlatBufferDecodeError {}

// ── Builder helpers (used by macro-generated code) ──────────────────

/// Helper to create a FlatBuffer string vector from a slice of strings.
///
/// Public because macro-generated code calls this.
pub fn create_string_vector<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    strings: &[String],
) -> flatbuffers::WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>> {
    let offsets: Vec<_> = strings
        .iter()
        .map(|s| builder.create_string(s))
        .collect();
    builder.create_vector(&offsets)
}

/// VTable offset for field at given index.
///
/// FlatBuffer vtable layout: field N has offset `4 + 2*N`.
/// (First 4 bytes of vtable are vtable_size and table_data_size.)
pub const fn vt_offset(field_index: usize) -> flatbuffers::VOffsetT {
    (4 + 2 * field_index) as flatbuffers::VOffsetT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_default_is_json() {
        assert_eq!(Format::default(), Format::Json);
    }

    #[test]
    fn format_from_accept() {
        assert_eq!(Format::from_accept("application/json"), Format::Json);
        assert_eq!(
            Format::from_accept("application/x-flatbuffers"),
            Format::FlatBuffers
        );
        assert_eq!(
            Format::from_accept("application/x-flatbuffers, application/json"),
            Format::FlatBuffers
        );
        // Unknown defaults to JSON.
        assert_eq!(Format::from_accept("text/html"), Format::Json);
        assert_eq!(Format::from_accept("*/*"), Format::Json);
    }

    #[test]
    fn format_mime_strings() {
        assert_eq!(Format::Json.mime(), "application/json");
        assert_eq!(Format::FlatBuffers.mime(), "application/x-flatbuffers");
    }

    #[test]
    fn vt_offset_calculation() {
        assert_eq!(vt_offset(0), 4);
        assert_eq!(vt_offset(1), 6);
        assert_eq!(vt_offset(2), 8);
        assert_eq!(vt_offset(5), 14);
    }
}
