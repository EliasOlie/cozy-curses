#!/bin/bash
# Runner do cargo para wasm: o .wasm sai com centenas de MB de debuginfo e nomes
# de símbolos, o que deixa o wasm-bindgen/servidor lentíssimos. Removemos antes de servir.
set -euo pipefail

wasm="$1"
shift
stripped="${wasm%.wasm}.stripped.wasm"

# (não use --strip-all: ele apaga as seções customizadas que o wasm-bindgen lê)
llvm-strip --strip-debug --remove-section=name -o "$stripped" "$wasm"
exec wasm-server-runner "$stripped" "$@"
