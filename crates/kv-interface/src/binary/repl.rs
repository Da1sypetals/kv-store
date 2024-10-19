use clap::Parser;
use kv_interface::ksis::repl::main::repl_main;

#[derive(Parser)]
struct CliArgs {
    #[arg(short, long)]
    path: String,
}

fn main() {
    let args = CliArgs::parse();
    repl_main(&args.path);
}
