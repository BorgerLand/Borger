#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" != "ptlaaxobimwroe" ]; then
	echo "ERROR: Please use the borger CLI tool instead of invoking this script directly." >&2
	exit 1
fi

shift

IDE_CONFIG_SOURCES=(".vscode/launch.json.example" ".vscode/settings.json.example")
IDE_CONFIG_TARGETS=(".vscode/launch.json"         ".vscode/settings.json")
YES_RE="^([yY][eE][sS]|[yY])$"

SERVER_BUILD_ARGS="--profile server-dev --features server"
SERVER_CMD="./server --devcert ../../assets/devcert.json"
CLIENT_DIR="borger/client/rs"
CLIENT_PKG="$CLIENT_DIR/pkg"

pre_launch_checks()
{
	if pgrep -f "RUN-CODEGEN" >/dev/null 2>&1; then
		echo "ERROR: Please close borger dev to avoid accidentally triggering concurrent builds." >&2
		exit 1
	fi
	
	NVM_DIR="$([ -z "${XDG_CONFIG_HOME-}" ] && printf %s "${HOME}/.nvm" || printf %s "${XDG_CONFIG_HOME}/nvm")"
	\. "$NVM_DIR/nvm.sh" --no-use
}

toolchain_check()
{
	if [[ ! -d node_modules ]]; then
		echo "ERROR: Not so fast. Please run \`borger install\` first." >&2
		exit 1
	fi
	
	local rust_said rust node borger_date borger_info
	local rust_re='rustc ([^ ]+) \([0-9a-f]+ ([0-9-]+)\)'

	nvm use > /dev/null

	node="${NVM_BIN:-}"
	node="${node%/bin}"
	node="${node##*/}"
	node="${node#v}"

	rust_said=$(rustup --version 2>&1) || rust_said=""

	if [[ "$rust_said" =~ $rust_re ]]; then
		rust="${BASH_REMATCH[2]} ${BASH_REMATCH[1]}"
	fi

	borger_commit=$(git -C borger rev-parse --short HEAD)
	borger_date=$(git -C borger log -1 --format=%cd --date=short)
	if [[ -n "$(git -C borger status --porcelain --untracked-files=all)" ]]; then
		borger_info="${borger_commit} (modified)"
	else
		borger_info="${borger_commit} ${borger_date}"
	fi

	echo "Toolchain  -  Borger ${borger_info}  -  Rust ${rust:-unknown}  -  Node.js ${node:-unknown} "

	run_codegen
}

run_codegen()
{
	cd borger/code_generator
	npx tsc
	npx tsx src/main.ts
	cd ../..
}

cmd_postinit()
{
	pre_launch_checks
	nvm install
	
	set -x
	
	rm -r .github eslint.config.ts prettier.config.ts rustfmt.toml
	npm uninstall @eslint/js eslint-config-prettier eslint-plugin-prettier prettier typescript-eslint
	git remote remove origin
	git checkout --orphan blank-history
	git add .
	GIT_AUTHOR_NAME="The Borger Monster" GIT_AUTHOR_EMAIL="monster@borger.land" \
	GIT_COMMITTER_NAME="The Borger Monster" GIT_COMMITTER_EMAIL="monster@borger.land" \
	git commit -m "Initial commit"
	git branch -D master
	git branch -m master
	
	cmd_install false
}

cmd_install()
{
	local run_pre_launch="${1:-true}"
	if [[ "$run_pre_launch" == "true" ]]; then
		pre_launch_checks
		nvm install
	fi
	
	set -x
	npm ci
	
	for i in "${!IDE_CONFIG_SOURCES[@]}"; do
		local src="${IDE_CONFIG_SOURCES[$i]}"
		local target="${IDE_CONFIG_TARGETS[$i]}"
		[ -f "$target" ] || cp "$src" "$target"
	done
	
	#build it for the first time so that dev doesn't kick off even more setting up
	run_codegen
	cd borger/server
	cargo build $SERVER_BUILD_ARGS
	cd ../client/rs
	wasm-pack build --out-name client_rs_mt --no-opt --target=web --profile client-dev --features client --config 'include=[".cargo/config.mt.toml"]'
	
	set +x
	echo
	echo "Done. When you're ready to rumble, do:"
	[ -n "${DIR:-}" ] && echo "  cd \"$DIR\""
	echo "  borger dev"
}

cmd_dev_help()
{
	echo "Usage: borger dev [options]"
	echo ""
	echo "Options:"
	echo "  --session-replay  Enable client-sided session recording+dumping+replaying"
	echo "  --singlethreaded  Build the client in singlethreaded mode, simulating a game portal's restrictive HTTP server (Poki, CrazyGames, etc.)"
	echo "  --host            Allow other devices to connect to this device's dev server"
	echo "  --help, -h, help"
}

. "$(dirname "${BASH_SOURCE[0]}")/cli_dev_status_bar.sh"
cmd_dev()
{
	local CLIENT_FEATURES="client"
	local CLIENT_VARIANT="mt"
	local VITE_ENV=""
	local VITE_HOST=""

	while [[ $# -gt 0 ]]; do
		case $1 in
			--session-replay)
				CLIENT_FEATURES="$CLIENT_FEATURES,session_replay"
				shift
				;;
			--singlethreaded)
				#the wasm and the headers have to agree: this build has no shared memory
				#to hand out, and the multithreaded one cannot load without it
				CLIENT_FEATURES="$CLIENT_FEATURES,singlethreaded"
				CLIENT_VARIANT="st"
				VITE_ENV="BORGER_SINGLETHREADED=1"
				shift
				;;
			--host)
				VITE_HOST="--host"
				shift
				;;
			help|--help|-h)
				cmd_dev_help
				exit 0
				;;
			*)
				cmd_dev_help
				exit 1
				;;
		esac
	done
	
	pre_launch_checks
	toolchain_check
	
	if sb_is_supported; then
		trap 'sb_close' EXIT
		sb_open
	fi
	
	local -a DEV_CMD=(
		npx concurrently
		--names "RUN-CODEGEN,TSC-CODEGEN,SERVER-RUST,CLIENT-RUST,CLIENT-VITE"
		-c      "bgGreen.bold,bgYellow.bold,bgBlue.bold,bgWhite.bold,bgRed.bold"
		"cd borger/code_generator && npx tsx watch --clear-screen=false src/main.ts"
		"cd borger/code_generator && npx tsc --watch --preserveWatchOutput"
		"cd borger/server    && cargo watch --why --no-vcs-ignores \
			-w '../../borger/engine' \
			-w '../../borger/server' \
			-w '../../borger/procmac' \
			-w '../../src/simulation' \
			-w '../../Cargo.toml' \
			-w '../../Cargo.lock' \
			-w '../../rust-toolchain.toml' \
			-s '       cargo build $SERVER_BUILD_ARGS \
				&& cd ../../target/server-dev \
				&& while true; do RUST_BACKTRACE=full $SERVER_CMD; sleep 1; done'"
		"cd $CLIENT_DIR && cargo watch --why --no-vcs-ignores \
			-w '../../../borger/engine' \
			-w '../../../borger/client/rs/src' \
			-w '../../../borger/client/rs/Cargo.toml' \
			-w '../../../borger/client/rs/.cargo' \
			-w '../../../borger/procmac' \
			-w '../../../src/simulation' \
			-w '../../../Cargo.toml' \
			-w '../../../Cargo.lock' \
			-w '../../../rust-toolchain.toml' \
			-s \"rm -f pkg/*.wasm \
				&& wasm-pack build --out-name client_rs_$CLIENT_VARIANT --no-opt --target=web --profile client-dev --features $CLIENT_FEATURES \
				--config 'include=[\\\".cargo/config.$CLIENT_VARIANT.toml\\\"]'\""
		"$VITE_ENV npx vite $VITE_HOST"
	)
	
	if sb_is_supported; then
		FORCE_COLOR=1 "${DEV_CMD[@]}" 2>&1 | sb_loop
	else
		"${DEV_CMD[@]}"
	fi
}

cmd_release()
{
	pre_launch_checks
	toolchain_check
	
	if [ -d "release" ] || [ -d "$CLIENT_PKG" ]; then
		echo "WARNING: The existing /release and /$CLIENT_PKG folders need to be removed to proceed."
		echo "Are you sure you want to obliterate? (y/n)"
		
		read -r response
		if ! [[ "$response" =~ $YES_RE ]]; then
			exit 1
		fi
		
		set -x
		rm -rf release "$CLIENT_PKG"
	else
		set -x
	fi
	
	cd borger/client/rs
	wasm-pack build --out-name client_rs_mt --profile client-release --target=web --features client --config 'include=[".cargo/config.mt.toml"]'
	wasm-pack build --out-name client_rs_st --profile client-release --target=web --features client,singlethreaded  --config 'include=[".cargo/config.st.toml"]'
	cd ../../..
	npx vite build
	npx rolldown --minify "$CLIENT_PKG"/client_rs*.js -d release/client/assets
	mv "$CLIENT_PKG"/*.wasm release/client/assets
	rm -rf release/client/devcert.json "$CLIENT_PKG" #leaving stale builds in pkg confuses wasm-opt sometimes
	cd borger/server
	cargo build --profile server-release --features server
	cd  ../..
	mv target/server-release/server release
}

cmd_clean()
{
	pre_launch_checks
	
	#for whatever reason, git clean -X (delete gitignored files) and -e (exclude) can't
	#be used together, so excluded files must be manually filtered from the dry run
	local GITIGNORED
	GITIGNORED=$(
		git clean -dffXn | sed -n 's/^Would remove //p'
		git submodule foreach --recursive --quiet \
			'git clean -dffXn | sed -n "s|^Would remove |$displaypath/|p"'
	)
	
	local FILES_TO_CLEAN=()
	while IFS= read -r file; do
		[[ -n "$file" ]] || continue
		
		local is_protected=false
		for pattern in "${IDE_CONFIG_TARGETS[@]}"; do
			if [[ "$file" == "$pattern" ]]; then
				is_protected=true
				break
			fi
		done
		
		if [[ "$is_protected" == false ]]; then
			FILES_TO_CLEAN+=("$file")
		fi
	done <<< "$GITIGNORED"
	
	if [[ ${#FILES_TO_CLEAN[@]} -eq 0 ]]; then
		echo "Workspace is already clean. Nothing to delete."
		exit 0
	fi
	
	for file in "${FILES_TO_CLEAN[@]}"; do
		echo "Would remove $file"
	done
	
	echo "WARNING: The files listed above (everything in .gitignore excluding IDE config) will be deleted."
	echo "Are you sure you want to obliterate? (y/n)"
	
	read -r response
	if ! [[ "$response" =~ $YES_RE ]]; then
		exit 1
	fi
	
	for file in "${FILES_TO_CLEAN[@]}"; do
		rm -rf "$file"
	done
}

cmd_help()
{
	echo "  install        Download and compile this repo's dependencies"
	echo "  dev [options]  Development mode: automatically rebuilds when code changes"
	echo "  release        Release mode: creates server executable and static client webpage"
	echo "  clean          Remove all gitignored files except IDE config"
}

case "${1:-}" in
	postinit)
		shift
		cmd_postinit "$@"
		;;
	install)
		shift
		cmd_install "$@"
		;;
	dev)
		shift
		cmd_dev "$@"
		;;
	release)
		shift
		cmd_release "$@"
		;;
	clean)
		shift
		cmd_clean "$@"
		;;
	help|--help|-h)
		cmd_help
		;;
	*)
		borger ptlaaxobimwroe help
		exit 1
		;;
esac
