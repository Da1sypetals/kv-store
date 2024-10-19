# Kv-Store
An implementation of kv-store storage.

## Build
```
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