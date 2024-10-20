use crate::service::{parse::parse_path, response::ToResp};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use kv_interface::{interface::dirstore::DirStore, ksis::parse::commands::Command};
use log::{error, info};
use std::{error, sync::Arc};

/// ParseError and ExecError are two types of errors that should respond to client
pub async fn get(
    State(store): State<Arc<DirStore<'_>>>,
    Path(path): Path<String>,
) -> (StatusCode, String) {
    match parse_path(path) {
        Ok(dir) => {
            info!("GET dir: {}", dir);
            let cmd_str = format!("$get {}", dir);
            {
                match Command::try_parse(cmd_str) {
                    Ok(cmd) => {
                        //
                        store.exec_command(cmd).to_resp()
                    }
                    Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
                }
            }
        }
        Err(msg) => {
            error!("Invalid path: {}", msg);
            (StatusCode::BAD_REQUEST, format!("Invalid path: {}", msg))
        }
    }
}
