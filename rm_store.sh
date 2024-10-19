#!/bin/bash

STORE_DIR="store"

# 定义红色输出
RED='\033[0;31m'
NC='\033[0m' # No Color

echo


# 检查是否指定了 --all 参数
if [[ "$1" == "--all" ]]; then
    echo "===== store remove: all ======"
    echo
    echo "Deleting all contents in $STORE_DIR..."
    rm -rf "$STORE_DIR"/*
    echo
    echo "===== store remove: done ====="
    echo
    exit 0
fi

# 如果没有指定 --all 参数，则处理其他参数
echo
echo "===== store remove: selected  ====="
echo
for DIR in "$@"; do
    if [[ -d "$STORE_DIR/$DIR" ]]; then
        echo "Deleting directory: $STORE_DIR/$DIR..."
        rm -rf "$STORE_DIR/$DIR"
    else
        echo -e "${RED}Error: Directory $STORE_DIR/$DIR not found.${NC}" >&2
    fi
    echo
done
echo "======= store remove: done ========"
echo
