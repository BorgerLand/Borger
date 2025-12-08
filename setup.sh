#!/bin/bash
set -ex

#wasm-pack has some unreleased features regarding custom profiles
#https://github.com/drager/wasm-pack/pull/1489
cargo install --git https://github.com/Argeo-Robotics/wasm-pack.git --rev 956f6e4 --locked
cargo install cargo-watch --locked --version 8.5.3
bun install --frozen-lockfile

cp --update=none rust-analyzer.toml.example rust-analyzer.toml
cp --update=none .vscode/settings.json.example .vscode/settings.json

#build it for the first time so that dev.sh doesn't kick off even more setting up
cd engine/code_generator
bun src/Main.ts
cd ../server
cargo build --profile server-dev --features server
cd ../client/rs
wasm-pack build --no-opt --target=web --profile client-dev --features client

set +x
echo "
Now run ./dev.sh when you're ready to rumble"
