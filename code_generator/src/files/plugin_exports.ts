import { ENGINE_GENERATED_DIR, pluginCrateName, stateWarningBlock } from "@borger/code_generator/common.ts";
import fs from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generatePluginExports(flattened: FlattenedOutput) {
	fs.writeFileSync(
		`${ENGINE_GENERATED_DIR}/plugin_exports.rs`,
		`${stateWarningBlock()}

${[...new Set(flattened.plugins.map(pluginCrateName))].map((crateName) => `pub use ${crateName};`).join("\n")}
`,
	);
}
