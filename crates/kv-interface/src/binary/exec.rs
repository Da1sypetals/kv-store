use clap::{Args, Parser};
use kv_interface::ksis::exec::main::exec_main;

#[derive(Parser)]
struct CliArgs {
    #[arg(short, long)]
    path: String,
}

fn main() {
    let args = CliArgs::parse();
    exec_main(&args.path);
}
