# Kv-Store
An implementation of kv-store storage.

## Build
```bash
# build REPL
bash build.sh -r

# build command execution
bash build.sh -x

# ...or, build all
bash build.sh --all
```

## Run
- Modify configuration files. 
  - Try to modify `config.toml` to configure REPL;
  - Try to modify `execution.ksis.toml` to configure command execution.
  
- Run REPL: `./repl <config_path.toml>`
- Execute a set of commands: `./exec <exec_path.ksis.toml>`

## Command syntax
```
# crud
$put path.to.your.key. -t your_value
$get path.to.your.key.
$ls path.to.your.key.
# list all k-v pairs
$ls . 
$del path.to.your.key.

# batched
$bput batch_name path.to.your.key. -t your_value
$bdel batch_name path.to.your.key.

# batched
# create batch
$bat batch_name 
$bput batch_name path.to.your.key. -t your_value
$bdel batch_name path.to.your.key.
# commit batch
$cmt batch_name 

# merge compacts the inner storage
$mrg

```

## Use API

See `enum Commands` in `crates/kv-interface/ksis/parse/commands` for a complete list of commands.

## Use http interface

See `crates/kv-web-service/src/main.rs` for details.  
