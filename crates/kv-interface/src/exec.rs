use ksis::exec::{exec::Execution, main::exec_main};
mod interface;
mod ksis;

fn main() {
    exec_main("execution.ksis.toml");
}
