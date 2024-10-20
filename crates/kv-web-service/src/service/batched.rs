use crate::service::{parse::parse_path, response::ToResp};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use kv_interface::{interface::dirstore::DirStore, ksis::parse::commands::Command};
use log::{error, info};
use std::{collections::BTreeMap, error, sync::Arc};

#[axum::debug_handler]
pub async fn new(
    State(store): State<Arc<DirStore>>,
    Path(batchname): Path<String>,
) -> (StatusCode, String) {
    info!("New batch: {}", batchname);
    let cmd_str = format!("$bat {}", batchname);
    match Command::try_parse(cmd_str) {
        Ok(cmd) => store.exec_command(cmd).to_resp(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}

#[axum::debug_handler]
pub async fn commit(
    State(store): State<Arc<DirStore>>,
    Path(batchname): Path<String>,
) -> (StatusCode, String) {
    info!("Commit batch: {}", batchname);
    let cmd_str = format!("$cmt {}", batchname);
    match Command::try_parse(cmd_str) {
        Ok(cmd) => store.exec_command(cmd).to_resp(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}

#[axum::debug_handler]
pub async fn delete(
    State(store): State<Arc<DirStore>>,
    Path((batchname, path)): Path<(String, String)>,
) -> (StatusCode, String) {
    match parse_path(path) {
        Ok(dir) => {
            info!("DELETE dir: {}", dir);
            let cmd_str = format!("$bdel {} {}", batchname, dir);
            match Command::try_parse(cmd_str) {
                Ok(cmd) => store.exec_command(cmd).to_resp(),
                Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            }
        }
        Err(msg) => {
            error!("Invalid path: {}", msg);
            (StatusCode::BAD_REQUEST, format!("Invalid path: {}", msg))
        }
    }
}

#[axum::debug_handler]
pub async fn put(
    State(store): State<Arc<DirStore>>,
    Path((batchname, path)): Path<(String, String)>,
    Json(body): Json<BTreeMap<String, String>>,
) -> (StatusCode, String) {
    match parse_path(path) {
        Ok(dir) => {
            info!("POST dir: {}", dir);
            let value_type = match body.get("value_type") {
                Some(value_type) => value_type,
                None => return (StatusCode::BAD_REQUEST, "Require field: value_type".into()),
            };
            let value = match body.get("value") {
                Some(value) => value,
                None => return (StatusCode::BAD_REQUEST, "Require field: value".into()),
            };

            // do not forget the dash -
            let cmd_str = format!("$bput {} {} -{} {}", batchname, dir, value_type, value);

            match Command::try_parse(cmd_str) {
                Ok(cmd) => store.exec_command(cmd).to_resp(),
                Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            }
        }
        Err(msg) => {
            error!("Invalid path: {}", msg);
            (StatusCode::BAD_REQUEST, format!("Invalid path: {}", msg))
        }
    }
}
