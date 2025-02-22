use crate::interface::config::{start_dir_store, DirStoreConfig};
use crate::interface::dirstore::DirStore;
use crate::ksis::parse::commands::Command;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

pub fn repl_main(config_path: &str) -> Result<()> {
    // `()` can be used when no completer is required
    let mut rl = DefaultEditor::new()?;

    // init kv store
    // let config = DirStoreConfig::from_toml(config_path.into());
    // let config = match config {
    //     Ok(config) => config,
    //     Err(e) => {
    //         eprintln!("Failed to start kv store: {}", e.to_string());
    //         std::process::exit(0);
    //     }
    // };
    // let ds = match DirStore::open(config) {
    //     Ok(ds) => ds,
    //     Err(e) => {
    //         eprintln!("Directory storage initialization failed: {}", e.to_string());
    //         std::process::exit(1);
    //     }
    // };

    let ds = start_dir_store(config_path);

    loop {
        let readline = rl.readline("kv > ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                // execute line
                match Command::try_parse(line) {
                    Ok(cmd) => {
                        //
                        match ds.exec_command(cmd) {
                            Ok(result) => {
                                //
                                println!("{}", result.to_string());
                            }
                            Err(e) => {
                                //
                                println!("[error] {}", e.to_string())
                            }
                        }
                    }
                    Err(e) => {
                        //
                        println!("[error] {}", e.to_string())
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Ctrl-C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Ctrl-D");
                break;
            }
            Err(err) => {
                println!("CLI error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
