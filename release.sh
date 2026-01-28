#!/bin/bash
set -e

BUILD=""
RUN=""
PASSWORD=""
FULLCHAIN=""
PRIVKEY=""
PORT="5173"

if [ $# -eq 0 ]; then
	echo "Options:"
	echo "--build"
	echo "--run"
	echo "--password \"the password\""
	echo "--fullchain \"/path/to/fullchain.pem\""
	echo "--privkey \"/path/to/privkey.pem\""
	echo "--port # (default: 5173)"
	echo "--help"
	exit 0
fi

while [[ $# -gt 0 ]]; do
	case $1 in
		--build)
			BUILD=1
			shift
			;;
		--run)
			RUN=1
			shift
			;;
		--password)
			PASSWORD="$2"
			shift 2
			;;
		--fullchain)
			FULLCHAIN="$2"
			shift 2
			;;
		--privkey)
			PRIVKEY="$2"
			shift 2
			;;
		--port)
			PORT="$2"
			shift 2
			;;
		-h|--help|*)
			echo "Options:"
			echo "--build"
			echo "--run"
			echo "--password \"the password\""
			echo "--fullchain \"/path/to/fullchain.pem\""
			echo "--privkey \"/path/to/privkey.pem\""
			echo "--port <port> (default: 5173)"
			echo "--help"
			exit 0
			;;
	esac
done

if [ -n "$BUILD" ]; then
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
	rm -f release/client/devcert.json
	mv target/server-release/server release
	cd release/client/assets
	find . -name "*.js" -type f -exec sed -i 's/from\s*["'\''"]@engine\/client_rs["'\''\"]/from".\/client_rs.js"/g' {} \;
	cd ../../..
	
	set +x
fi

if [ -n "$RUN" ]; then
	# Validate that both or neither of fullchain/privkey are set
	if [ -n "$FULLCHAIN" ] && [ -z "$PRIVKEY" ]; then
		echo "Error: --fullchain requires --privkey to also be set"
		exit 1
	fi
	if [ -n "$PRIVKEY" ] && [ -z "$FULLCHAIN" ]; then
		echo "Error: --privkey requires --fullchain to also be set"
		exit 1
	fi
	
	# Convert to absolute paths before cd
	if [ -n "$FULLCHAIN" ]; then
		FULLCHAIN="$(cd "$(dirname "$FULLCHAIN")" && pwd)/$(basename "$FULLCHAIN")"
		PRIVKEY="$(cd "$(dirname "$PRIVKEY")" && pwd)/$(basename "$PRIVKEY")"
	fi
	
	cd release
	
	if [ -n "$FULLCHAIN" ]; then
		SERVER_CMD="./server -p 6969 --fullchain \"$FULLCHAIN\" --privkey \"$PRIVKEY\""
	else
		SERVER_CMD="./server -p 6969 --devcert client/devcert.json"
	fi
	
	bun concurrently \
		"$SERVER_CMD" \
		"cd client && cat <<'SCRIPT' | PASSWORD=\"$PASSWORD\" FULLCHAIN=\"$FULLCHAIN\" PRIVKEY=\"$PRIVKEY\" PORT=\"$PORT\" bun -
import { serve } from \"bun\";
import { file } from \"bun\";

const password = process.env.PASSWORD;
const fullchain = process.env.FULLCHAIN;
const privkey = process.env.PRIVKEY;
const port = parseInt(process.env.PORT) || 5173;
const headers = {
	\"Cross-Origin-Opener-Policy\": \"same-origin\",
	\"Cross-Origin-Embedder-Policy\": \"require-corp\",
	\"Cache-Control\": \"no-store\",
	\"Strict-Transport-Security\": \"max-age=31536000; includeSubDomains\",
};

function checkAuth(req) {
	if (!password) return true;
	const auth = req.headers.get(\"Authorization\");
	if (!auth || !auth.startsWith(\"Basic \")) return false;
	const decoded = atob(auth.slice(6));
	const [, pwd] = decoded.split(\":\");
	return pwd === password;
}

const serverOptions = {
	port,
	hostname: \"0.0.0.0\",
	async fetch(req) {
		if (!checkAuth(req)) {
			return new Response(\"Unauthorized\", {
				status: 401,
				headers: { ...headers, \"WWW-Authenticate\": \"Basic realm=\\\"Protected\\\"\" },
			});
		}
		
		const url = new URL(req.url);
		let pathname = url.pathname;

		if (pathname === \"/\") pathname = \"/index.html\";

		//remove leading slash for file path
		const filePath = pathname.slice(1) || \"index.html\";

		const fileHandle = file(filePath);

		if (await fileHandle.exists()) return new Response(fileHandle, { headers });
		return new Response(\"404 Not Found\", { status: 404, headers });
	},
};

if (fullchain && privkey) {
	console.log(\"Loaded HTTPS/TLS certificates\");
	serverOptions.tls = {
		cert: Bun.file(fullchain),
		key: Bun.file(privkey),
	};
}

const protocol = (fullchain && privkey) ? \"HTTPS\" : \"HTTP\";
console.log(\`\${protocol} server running on port \${port}\`);

serve(serverOptions);
SCRIPT"
	exit 0
fi
