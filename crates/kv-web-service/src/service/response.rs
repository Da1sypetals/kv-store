use axum::http::StatusCode;
use kv_interface::interface::errors::{ExecOutput, ExecResult, ExecReturn};

pub trait ToResp {
    fn to_resp(self) -> (StatusCode, String);
}

impl ToResp for ExecReturn {
    fn to_resp(self) -> (StatusCode, String) {
        match self {
            Ok(out) => (StatusCode::OK, out.to_string()),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()),
        }
    }
}
