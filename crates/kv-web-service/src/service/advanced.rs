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

pub async fn exec(
    State(store): State<Arc<DirStore>>,
    Json(body): Json<BTreeMap<String, String>>,
) -> (StatusCode, String) {
    info!("Execute command...");

    // do not forget the dash -
    let cmd_str = match body.get("command") {
        Some(command) => command.clone(),
        None => return (StatusCode::BAD_REQUEST, "Require field: command".into()),
    };

    match Command::try_parse(cmd_str) {
        Ok(cmd) => store.exec_command(cmd).to_resp(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn merge(State(store): State<Arc<DirStore>>) -> (StatusCode, String) {
    match store.exec_command(Command::Merge) {
        Ok(res) => (StatusCode::OK, res.to_string()),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}
