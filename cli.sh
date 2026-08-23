#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" != "ptlaaxobimwroe" ]; then
	echo "ERROR: Please use the borger CLI tool instead of invoking this script directly."
	exit 1
fi

shift

IDE_CONFIG_SOURCES=(".vscode/launch.json.example" ".vscode/settings.json.example")
IDE_CONFIG_TARGETS=(".vscode/launch.json"         ".vscode/settings.json")
YES_RE="^([yY][eE][sS]|[yY])$"

pre_launch_checks()
{
	if pgrep -f "RUN-CODEGEN" >/dev/null 2>&1; then
		echo "ERROR: Please close borger dev to avoid accidentally triggering concurrent builds."
		exit 1
	fi
	
	NVM_DIR="$([ -z "${XDG_CONFIG_HOME-}" ] && printf %s "${HOME}/.nvm" || printf %s "${XDG_CONFIG_HOME}/nvm")"
	\. "$NVM_DIR/nvm.sh" --no-use
}

cmd_postinit()
{
	pre_launch_checks
	nvm install
	
	set -x
	
	rm eslint.config.ts prettier.config.ts rustfmt.toml
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
		set -x
	fi
	
	git submodule update --init --recursive
	npm ci
	
	for i in "${!IDE_CONFIG_SOURCES[@]}"; do
		local src="${IDE_CONFIG_SOURCES[$i]}"
		local target="${IDE_CONFIG_TARGETS[$i]}"
		[ -f "$target" ] || cp "$src" "$target"
	done
	
	#build it for the first time so that dev doesn't kick off even more setting up
	cd borger/code_generator
	npx tsc
	npx tsx src/main.ts
	cd ../server
	cargo build --profile server-dev --features server
	cd ../client/rs
	wasm-pack build --out-name client_rs_mt --no-opt --target=web --profile client-dev --features client --config 'include=[".cargo/config.mt.toml"]'
	
	set +x
	echo
	echo "Done. When you're ready to rumble, do:"
	[ -n "${DIR:-}" ] && echo "  cd \"$DIR\""
	echo "  borger dev"
}

dev_help()
{
	echo "Usage: borger dev [options]"
	echo ""
	echo "Options:"
	echo "  --session-replay  Enable client-sided session recording+dumping+replaying"
	echo "  --host            Allow other devices to connect to this device's dev server"
	echo "  --help, -h, help"
}

cmd_dev()
{
	pre_launch_checks
	
	local CLIENT_FEATURES="client"
	local VITE_HOST=""
	
	while [[ $# -gt 0 ]]; do
		case $1 in
			--session-replay)
				CLIENT_FEATURES="$CLIENT_FEATURES,session_replay"
				shift
				;;
			--host)
				VITE_HOST="--host"
				shift
				;;
			help|--help|-h)
				dev_help
				exit 0
				;;
			*)
				dev_help
				exit 1
				;;
		esac
	done
	
	nvm use
	cd borger/code_generator
	npx tsc
	npx tsx src/main.ts
	cd ../..
	
	npx concurrently \
		--names "RUN-CODEGEN,TSC-CODEGEN,SERVER-RUST,CLIENT-RUST,CLIENT-VITE" \
		-c      "bgGreen.bold,bgYellow.bold,bgBlue.bold,bgWhite.bold,bgRed.bold" \
		"cd borger/code_generator && npx tsx watch --clear-screen=false src/main.ts" \
		"cd borger/code_generator && npx tsc --watch --preserveWatchOutput" \
		"cd borger/server    && cargo watch --why --no-vcs-ignores \
			-w '../../borger/engine' \
			-w '../../borger/server' \
			-w '../../borger/procmac' \
			-w '../../src/simulation' \
			-w '../../Cargo.toml' \
			-w '../../Cargo.lock' \
			-w '../../rust-toolchain.toml' \
			-s 'cargo        build                                               --profile server-dev --features server \
				&& cd ../../target/server-dev \
				&& while true; do RUST_BACKTRACE=full ./server --devcert ../../assets/devcert.json; sleep 1; done'" \
		"cd borger/client/rs && cargo watch --why --no-vcs-ignores \
			-w '../../../borger/engine' \
			-w '../../../borger/client/rs/src' \
			-w '../../../borger/client/rs/Cargo.toml' \
			-w '../../../borger/client/rs/.cargo' \
			-w '../../../borger/procmac' \
			-w '../../../src/simulation' \
			-w '../../../Cargo.toml' \
			-w '../../../Cargo.lock' \
			-w '../../../rust-toolchain.toml' \
			-s \"rm -f pkg/client_rs_mt_bg.wasm \
				&& wasm-pack build --out-name client_rs_mt --no-opt --target=web --profile client-dev --features $CLIENT_FEATURES \
				--config 'include=[\\\".cargo/config.mt.toml\\\"]'\"" \
		"npx vite $VITE_HOST"
}

cmd_release()
{
	pre_launch_checks
	nvm use
	
	if [ -d "release" ] || [ -d "borger/client/rs/pkg" ]; then
		echo "WARNING: The existing /release and /borger/client/rs/pkg folders need to be removed to proceed."
		echo "Are you sure you want to obliterate? (y/n)"
		
		read -r response
		if ! [[ "$response" =~ $YES_RE ]]; then
			exit 1
		fi
		
		set -x
		rm -rf release borger/client/rs/pkg
	else
		set -x
	fi
	
	cd borger/code_generator
	npx tsc
	npx tsx src/main.ts
	cd ../client/rs
	wasm-pack build --out-name client_rs_mt --profile client-release --target=web --features client --config 'include=[".cargo/config.mt.toml"]'
	wasm-pack build --out-name client_rs_st --profile client-release --target=web --features client,singlethreaded  --config 'include=[".cargo/config.st.toml"]'
	cd ../../..
	npx vite build
	npx rolldown --minify borger/client/rs/pkg/client_rs*.js -d release/client/assets
	mv borger/client/rs/pkg/*.wasm release/client/assets
	rm -rf release/client/devcert.json borger/client/rs/pkg #leaving stale builds in pkg confuses wasm-opt sometimes
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

main_help()
{
	echo "  install        Download and compile dependencies"
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
		main_help
		;;
	*)
		borger ptlaaxobimwroe help
		exit 1
		;;
esac
