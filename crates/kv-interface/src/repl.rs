use ksis::{exec::Execution, repl::main::repl_main};
mod interface;
mod ksis;

fn main() {
    repl_main("config.toml");
}
