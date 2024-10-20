use super::{
    errors::{ParseError, ParseResult},
    parse::{enforce_batch_identifier, split_by_whitespace, split_first_space},
};
use crate::interface::{
    data_structure::{directory::Directory, value::Value},
    dirstore,
};
use regex::Regex;

// string -> command, error
#[derive(Debug)]
pub enum Command {
    Get {
        key: Directory,
    },
    Put {
        key: Directory,
        value: Value,
    },
    Delete {
        key: Directory,
    },
    List {
        // prefix
        key: Directory,
    },
    MakeBatch {
        batchname: String,
    },
    BatchedPut {
        key: Directory,
        batchname: String,
        value: Value,
    },
    BatchedDelete {
        batchname: String,
        key: Directory,
    },
    BatchCommit {
        batchname: String,
    },
    Merge,
}

impl Command {
    // parse command string into command data structure, deal with all parse failure
    pub fn try_parse(cmd_str: String) -> ParseResult<Self> {
        // This is important!
        let mut cmd_str = cmd_str.trim();
        // a command must start with $
        if cmd_str.chars().nth(0) != Some('$') {
            return Err(ParseError::InvalidSyntax {
                msg: "A command starts with `$`".into(),
            });
        }
        cmd_str = &cmd_str[1..];

        let cmd_parts = split_by_whitespace(cmd_str);
        if cmd_parts.is_empty() {
            return Err(ParseError::InvalidSyntax {
                msg: "Command is empty!".into(),
            });
        }
        let cmd_type = cmd_parts[0].as_str();
        let args = &cmd_parts[1..];

        match cmd_type {
            "get" => {
                // todo!()
                Self::try_parse_get(args)
            }
            "put" => {
                // todo!()
                Self::try_parse_put(args)
            }
            "del" => {
                // todo!()
                Self::try_parse_del(args)
            }
            "bput" => {
                // todo!()
                Self::try_parse_batched_put(args)
            }
            "bdel" => {
                // todo!()
                Self::try_parse_batched_del(args)
            }
            "ls" | "list" => {
                //
                Self::try_parse_list(args)
            }
            "cmt" | "commit" => {
                //
                Self::try_parse_batch_commit(args)
            }
            "bat" | "batch" => {
                //
                Self::try_parse_make_batch(args)
            }
            "mrg" | "merge" => {
                //
                Self::try_parse_merge(args)
            }
            _ => {
                //
                Err(ParseError::UnsupportedCommand {
                    cmd: cmd_type.to_string(),
                })
            }
        }
    }
}

impl Command {
    fn try_parse_dir(dir_str: &str) -> ParseResult<Directory> {
        if !is_valid_dir(dir_str) {
            return Err(ParseError::InvalidDirectory {
                src: dir_str.to_string(),
            });
        }
        let dirs: Vec<_> = dir_str
            .split('.')
            .map(|x| x.to_string())
            .filter(|x| x.len() != 0)
            .collect();

        Ok(Directory { dirs })
    }

    // `$get hello.world.baby.`
    fn try_parse_list(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 1)?;
        let dir = Self::try_parse_dir(&args[0])?;

        Ok(Command::List { key: dir })
    }

    // `$get hello.world.baby.`
    fn try_parse_get(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 1)?;
        let dir = Self::try_parse_dir(&args[0])?;

        Ok(Command::Get { key: dir })
    }

    // `$get hello.world.baby.`
    fn try_parse_del(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 1)?;
        let dir = Self::try_parse_dir(&args[0])?;

        Ok(Command::Delete { key: dir })
    }

    // `$get hello.world.baby.`
    fn try_parse_make_batch(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 1)?;
        let batchname = args[0].clone();
        enforce_batch_identifier(&batchname)?;

        Ok(Command::MakeBatch { batchname })
    }

    fn try_parse_merge(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 0)?;
        Ok(Command::Merge)
    }

    // `$get hello.world.baby.`
    fn try_parse_batched_del(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 2)?;

        let batchname = args[0].clone();
        enforce_batch_identifier(&batchname)?;

        let dir = Self::try_parse_dir(&args[1])?;

        Ok(Command::BatchedDelete {
            batchname,
            key: dir,
        })
    }

    /// s: string
    /// i: int
    /// r: real
    /// z: complex
    fn try_parse_put(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 3)?;
        let dir = Self::try_parse_dir(&args[0])?;
        let type_char = args[1].clone();
        let val_str = args[2].clone();

        let value = Value::parse(type_char, val_str)?;

        Ok(Command::Put { key: dir, value })
    }

    // `$get hello.world.baby.`
    fn try_parse_batched_put(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 3)?;

        let batchname = args[0].clone();
        enforce_batch_identifier(&batchname)?;

        let dir = Self::try_parse_dir(&args[1])?;

        let type_char = args[1].clone();
        let val_str = args[2].clone();

        let value = Value::parse(type_char, val_str)?;

        Ok(Command::BatchedPut {
            key: dir,
            value,
            batchname,
        })
    }

    // `$get hello.world.baby.`
    fn try_parse_batch_commit(args: &[String]) -> ParseResult<Command> {
        enforce_vec_len(args, 1)?;

        let batchname = args[0].clone();
        enforce_batch_identifier(&batchname)?;

        Ok(Command::BatchCommit { batchname })
    }
}

fn is_valid_dir(s: &str) -> bool {
    // 定义正则表达式，匹配英文字母、数字和英文句号
    let re = Regex::new(r"^[a-zA-Z0-9.]+$").unwrap();

    // 使用正则表达式匹配字符串
    re.is_match(s) && s.ends_with(".")
}

fn enforce_vec_len(args: &[String], expected: usize) -> ParseResult<()> {
    if args.len() != expected {
        Err(ParseError::IncompatibleArgCount {
            expected,
            found: args.len(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    #[should_panic(expected = "Invalid dir!")]
    fn test_parse_base() {
        let dir = "hello.world.baby.";
        let dir = Command::try_parse_dir(dir).unwrap();

        let dir = "hello.world.baby";
        let dir = Command::try_parse_dir(dir).expect("Invalid dir!");
    }

    #[test]
    fn test_parse_commands() {
        let cmd = "$get hello.world.baby.  ";
        let cmd = Command::try_parse(cmd.to_string()).unwrap();

        let cmd = "$get hello.world.baby.  aff ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "$del hello.world.baby.  ";
        let cmd = Command::try_parse(cmd.to_string()).unwrap();

        let cmd = "$del hello.world.baby.  aff ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "    $get  ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "$ls ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "$ls hello.world.";
        let cmd = Command::try_parse(cmd.to_string()).unwrap();

        let cmd = "$put hello.world.baby. hi  ";
        let cmd = Command::try_parse(cmd.to_string()).unwrap();

        let cmd = "$put hello.world.baby.  ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "$bput   b0  hello.world.baby. hi  ";
        let cmd = Command::try_parse(cmd.to_string()).unwrap();
        dbg!(cmd);

        let cmd = "$bput hello.world.baby. world ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "$bdel   b0  hello.world.baby.   ";
        let cmd = Command::try_parse(cmd.to_string()).unwrap();
        dbg!(cmd);

        let cmd = "$bdel   b0  hello.world.baby   ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());

        let cmd = "$bdel   b0  hello.world.baby. asdffs   ";
        let cmd = Command::try_parse(cmd.to_string());
        assert!(cmd.is_err());
    }
}
