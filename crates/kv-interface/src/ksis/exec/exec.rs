use crate::{
    interface::{config::DirStoreConfig, dirstore::DirStore},
    ksis::parse::commands::Command,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
};

#[derive(Serialize, Deserialize)]
pub struct KsisScript {
    commands: String,
    output_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Execution {
    pub config: DirStoreConfig,
    pub script: KsisScript,
}

impl Execution {
    pub fn new(config_path: &str) -> Result<Self, String> {
        if !config_path.ends_with(".ksis.toml") {
            return Err("File is expected to have extension `.ksis.toml`!".into());
        }
        let mut res: Self = toml::from_str(
            fs::read_to_string(config_path)
                .map_err(|e| e.to_string())?
                .as_str(),
        )
        .map_err(|e| e.to_string())?;

        res.script.commands = res.script.commands.trim().to_string();

        Ok(res)
    }

    /// Returns an error if failed to open dirstore, succees otherwise.
    ///  Error during execution is not caught here.
    pub fn execute(self) -> anyhow::Result<ExecutionResult> {
        let ds = DirStore::open(self.config)?;

        let result = self
            .script
            .commands
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                // filter away empty lines and comments
                if line.is_empty() || line.starts_with("#") {
                    None
                } else {
                    Some(line)
                }
            })
            .map(|line| {
                match Command::try_parse(line.into()) {
                    Ok(cmd) => {
                        //
                        match ds.exec_command(cmd) {
                            Ok(result) => {
                                //
                                format!("{}", result.to_string())
                            }
                            Err(e) => {
                                //
                                format!("[error] {}", e.to_string())
                            }
                        }
                    }
                    Err(e) => {
                        //
                        format!("[error] {}", e.to_string())
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ExecutionResult {
            result,
            output_path: self.script.output_path,
        })
    }
}

pub struct ExecutionResult {
    result: String,
    output_path: String,
}

impl ExecutionResult {
    pub fn save(self) {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(self.output_path)
            .expect("Failed to create result file!");

        file.write_all(self.result.as_bytes())
            .expect("Failed to write to result file!");
    }
}
