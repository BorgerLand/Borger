#!/bin/bash
set -e

if [ -d "release" ]; then
	echo "WARNING: The existing release folder needs to be removed to proceed."
	echo "Are you sure you want to obliterate? (y/n)"
	
	read -r response
	if ! [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
		exit 1
	fi
	
	rm -rf release
fi

set -x

mkdir -p release/client/assets
cd engine/code_generator
bun src/Main.ts
cd ../server
cargo build --profile server-release --features server
cd ../client/rs
wasm-pack build --profile client-release --target=web --features client
cd pkg
bun rolldown --minify client_rs.js -o client_rs.js
mv client_rs.js client_rs_bg.wasm ../../../../release/client/assets
cd  ../../../..
bun vite build
mv target/server-release/server release
cd release/client/assets
find . -name "*.js" -type f -exec sed -i 's/from\s*["'\''"]@engine\/client_rs["'\''\"]/from".\/client_rs.js"/g' {} \;

set +x

if ! echo "$@" | grep -q "\--run"; then
	echo "Use release.sh --run to also launch the build after it finishes cooking"
	exit 0
fi

cd ../..
bun concurrently \
	"./server -p 6969 --devcert client/devcert.json" \
	"cd client && cat <<'SCRIPT' | bun -
import { serve } from \"bun\";
import { file } from \"bun\";

const port = 5173;
const headers = {
	\"Cross-Origin-Opener-Policy\": \"same-origin\",
	\"Cross-Origin-Embedder-Policy\": \"require-corp\",
	\"Cache-Control\": \"no-store\",
};

serve({
	port,
	hostname: \"0.0.0.0\",
	async fetch(req) {
		const url = new URL(req.url);
		let pathname = url.pathname;

		if (pathname === \"/\") pathname = \"/index.html\";

		//remove leading slash for file path
		const filePath = pathname.slice(1) || \"index.html\";

		const fileHandle = file(filePath);

		if (await fileHandle.exists()) return new Response(fileHandle, { headers });
		return new Response(\"404 Not Found\", { status: 404, headers });
	},
});
SCRIPT"
