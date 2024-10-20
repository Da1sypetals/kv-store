use axum::extract::Path;
use regex::Regex;

pub fn parse_path(path: String) -> Result<String, String> {
    // check input path
    let path = path.trim_end();
    let re = Regex::new(r"^[a-zA-Z0-9/]+[^/]$").unwrap();
    if !re.is_match(&path) {
        return Err(format!("Invalid directory: {}", path));
    }

    let mut dir = path.trim().replace("/", ".");
    dir.push('.');

    Ok(dir)
}
