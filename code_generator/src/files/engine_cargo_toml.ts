import { ENGINE_DIR, rsFQNtoCrateName, stateWarningHash } from "@borger/code_generator/common.ts";
import fs from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";
import path from "path";

export function generateEngineCargoTOML(flattened: FlattenedOutput) {
	const uniqueCrates = new Map<string, string>(); //crate name, package name
	for (const plugin of flattened.plugins)
		uniqueCrates.set(rsFQNtoCrateName(plugin.rsSimulationFQN), plugin.nodePackageName);

	fs.writeFileSync(
		`${ENGINE_DIR}/Cargo.toml`,
		`${stateWarningHash()}

[package]
name = "borger"
version.workspace = true
edition.workspace = true

[features]
server = ["tokio"${uniqueCrates
			.keys()
			.map((crateName) => `, "${crateName}/server"`)
			.toArray()
			.join("")}]
client = ["atomicbox"${uniqueCrates
			.keys()
			.map((crateName) => `, "${crateName}/client"`)
			.toArray()
			.join("")}]
session_replay = ["dep:serde", "glam/serde"]
singlethreaded = []

[dependencies]
borger_procmac = { path = "../procmac" }
borger_plugin_sdk = { path = "../plugins/sdk/rs" }

log.workspace = true
glam.workspace = true

web-time = { version = "*", default-features = false }
wasm_thread = { git = "https://github.com/buttercrab/wasm_thread.git", branch = "patch-1", default-features = false, features = ["es_modules"] } #https://github.com/chemicstry/wasm_thread/pull/33

#plugins
${uniqueCrates
	.entries()
	.map(
		([crateName, packageName]) =>
			`${crateName} = { path = "${path.relative(`borger/engine`, fs.realpathSync(`node_modules/${packageName}`))}", optional = true }`,
	)
	.toArray()
	.join("\n")}

#server only
tokio = { version = "*", optional = true, default-features = false, features = ["sync"] }

#client only
atomicbox = { version = "*", optional = true, default-features = false }
serde = { version = "*", optional = true, default-features = false, features = ["std", "derive"] }
`,
	);
}
