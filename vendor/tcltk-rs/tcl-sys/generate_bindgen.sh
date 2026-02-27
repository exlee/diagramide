#!/bin/bash

SDK_PATH=$(xcrun --show-sdk-path)
TCL_INCLUDE="include"
TCL_HEADER="$TCL_INCLUDE/tcl.h"
LIBC_SHADOW="$TCL_INCLUDE/libc"
WIN_FIX="$TCL_INCLUDE/win_fix.h"
COMMON_FLAGS="--blocklist-type ssize_t --no-layout-tests"

targets=(
  "aarch64-apple-darwin"
  "aarch64-pc-windows-gnullvm"
  "aarch64-unknown-linux-musl"
  "x86_64-apple-darwin"
  "x86_64-pc-windows-gnu"
  "x86_64-unknown-linux-musl"
)

mkdir -p src/bindings

for target in "${targets[@]}"; do
  echo "Generating bindings for $target..."
  
  CLANG_TARGET=$target
  if [[ $target == "aarch64-pc-windows-gnullvm" ]]; then
      CLANG_TARGET="aarch64-pc-windows-gnu"
  fi

  if [[ $target == *"apple"* ]]; then
    EXTRA_CLANG="-isysroot $SDK_PATH -I$TCL_INCLUDE -D_DARWIN_C_SOURCE"
  else
    EXTRA_CLANG="-target $CLANG_TARGET -isystem $LIBC_SHADOW -I$TCL_INCLUDE -D__STDC__=1"
    
    if [[ $target == *"windows"* ]]; then
        # -fms-extensions is the key to letting Clang handle __int64 natively
        EXTRA_CLANG="-include $WIN_FIX -D_WIN32 -D_WIN64 -D_AMD64_ -fms-extensions $EXTRA_CLANG"
    fi
  fi

  bindgen "$TCL_HEADER" -o "src/bindings/${target}.rs" \
    $COMMON_FLAGS \
    -- $EXTRA_CLANG
done
