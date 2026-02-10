#!/bin/bash
set -e

#make sure code generator runs before anything else
cd engine/code_generator
bun src/main.ts
cd ../..

bun concurrently \
	--names "CODE--GENER,SERVER-RUST,CLIENT-RUST,CLIENT-VITE,TSC---CHECK" \
	-c	  "bgGreen.bold,bgBlue.bold,bgWhite.bold,bgRed.bold,bgYellow.bold" \
	"cd engine/code_generator && bun --watch --no-clear-screen src/main.ts" \
	"cd engine/server && cargo watch --why --no-vcs-ignores -s 'cargo build --profile server-dev --features server && cd ../../target/server-dev && while true; do RUST_BACKTRACE=full ./server -p 6969 --devcert ../../game/assets/devcert.json; sleep 1; done'" \
	"cd engine/client/rs && cargo watch --why --no-vcs-ignores -i 'pkg/*' -s 'wasm-pack build --no-opt --target=web --profile client-dev --features client'" \
	"bun vite" \
	"bun tsc --watch --preserveWatchOutput" \
