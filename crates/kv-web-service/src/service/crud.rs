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

/// ParseError and ExecError are two types of errors that should respond to client
pub async fn get(
    State(store): State<Arc<DirStore>>,
    Path(path): Path<String>,
) -> (StatusCode, String) {
    match parse_path(path) {
        Ok(dir) => {
            info!("GET dir: {}", dir);
            let cmd_str = format!("$get {}", dir);
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

pub async fn list_root(State(store): State<Arc<DirStore>>) -> (StatusCode, String) {
    info!("List root");
    let cmd_str = format!("$ls .");
    match Command::try_parse(cmd_str) {
        Ok(cmd) => store.exec_command(cmd).to_resp(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn list(
    State(store): State<Arc<DirStore>>,
    Path(path): Path<String>,
) -> (StatusCode, String) {
    match parse_path(path) {
        Ok(dir) => {
            info!("List dir: {}", dir);
            let cmd_str = format!("$ls {}", dir);
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

pub async fn delete(
    State(store): State<Arc<DirStore>>,
    Path(path): Path<String>,
) -> (StatusCode, String) {
    match parse_path(path) {
        Ok(dir) => {
            info!("DELETE dir: {}", dir);
            let cmd_str = format!("$del {}", dir);
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

pub async fn put(
    State(store): State<Arc<DirStore>>,
    Path(path): Path<String>,
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
            let cmd_str = format!("$put {} -{} {}", dir, value_type, value);

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
