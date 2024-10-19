use super::exec::Execution;

pub fn exec_main(path: &str) {
    let exec = match Execution::new(path) {
        Ok(exec) => exec,
        Err(e) => {
            eprintln!("{}", e.to_string());
            std::process::exit(1);
        }
    };

    let exec_res = match exec.execute() {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to open kv store: {}", e.to_string());
            std::process::exit(0);
        }
    };

    exec_res.save();
}
