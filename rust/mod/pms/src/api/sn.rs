use std::collections::HashMap;

use axum::{
    Router,
    extract::State,
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::model::SNSegment;
use crate::sn::SNConfig;
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sn/config", get(get_sn_config).put(set_sn_config))
        .route("/sn/encode", post(encode_sn))
        .route("/sn/decode", post(decode_sn))
        .route("/sn/segments", get(list_segments).post(upsert_segment))
        .route("/sn/dimensions", get(list_dimensions))
}

async fn get_sn_config(
    State(svc): State<AppState>,
) -> Result<Json<SNConfig>, ApiError> {
    ok_json(svc.get_sn_config())
}

async fn set_sn_config(
    State(svc): State<AppState>,
    Json(config): Json<SNConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.set_sn_config(&config).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncodeBody {
    model_no: u32,
    dimensions: HashMap<String, u32>,
    timestamp: Option<(u32, u32)>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EncodeResponse {
    raw: String,
    formatted: String,
    bytes: Vec<u8>,
}

async fn encode_sn(
    State(svc): State<AppState>,
    Json(body): Json<EncodeBody>,
) -> Result<Json<EncodeResponse>, ApiError> {
    let output = svc
        .encode_serial_number(body.model_no, body.dimensions, body.timestamp)
        .map_err(ApiError::from)?;
    Ok(Json(EncodeResponse {
        raw: output.raw,
        formatted: output.formatted,
        bytes: output.bytes,
    }))
}

#[derive(Deserialize)]
struct DecodeBody {
    sn: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecodeResponse {
    values: HashMap<String, u64>,
    model_no: Option<u64>,
    year: Option<u64>,
    week: Option<u64>,
    dimensions: HashMap<String, u64>,
}

async fn decode_sn(
    State(svc): State<AppState>,
    Json(body): Json<DecodeBody>,
) -> Result<Json<DecodeResponse>, ApiError> {
    let output = svc.decode_serial_number(&body.sn).map_err(ApiError::from)?;
    let model_no = output.model_no();
    let year = output.year();
    let week = output.week();
    let dimensions = output.dimensions();
    Ok(Json(DecodeResponse {
        values: output.values,
        model_no,
        year,
        week,
        dimensions,
    }))
}

#[derive(Deserialize)]
struct SegmentQuery {
    dimension: Option<String>,
}

async fn list_segments(
    State(svc): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<SegmentQuery>,
) -> Result<Json<Vec<SNSegment>>, ApiError> {
    ok_json(svc.list_sn_segments(q.dimension.as_deref()))
}

async fn upsert_segment(
    State(svc): State<AppState>,
    Json(segment): Json<SNSegment>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.upsert_sn_segment(&segment).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn list_dimensions(
    State(svc): State<AppState>,
) -> Result<Json<Vec<String>>, ApiError> {
    ok_json(svc.list_sn_dimensions())
}
