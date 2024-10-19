#!/bin/bash

# Check the number of arguments
if [ $# -eq 0 ]; then
    echo "Error: No arguments provided. Please use -x, -r, or --all."
    exit 1
fi

# Process arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        -x)
            echo "Compiling and copying the 'exec' executable..."
            cargo build --bin exec
            cp target/debug/exec .
            ;;
        -r)
            echo "Compiling and copying the 'repl' executable..."
            cargo build --bin repl
            cp target/debug/repl .
            ;;
        --all)
            echo "Compiling and copying all executables..."
            cargo build --bin exec
            cp target/debug/exec .
            cargo build --bin repl
            cp target/debug/repl .
            ;;
        *)
            echo "Error: Unknown argument \$1. Please use -x, -r, or --all."
            exit 1
            ;;
    esac
    shift
done

echo "Build completed."
