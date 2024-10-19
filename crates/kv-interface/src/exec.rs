use ksis::{exec::Execution, repl::main::repl_main};
mod interface;
mod ksis;

fn exec_main() {
    let path = "execution.ksis.toml";
    let exec = match Execution::new(path) {
        Ok(exec) => exec,
        Err(e) => {
            eprintln!("{}", e.to_string());
            std::process::exit(1);
        }
    };

    exec.execute().save();
}

fn main() {
    exec_main();
}
