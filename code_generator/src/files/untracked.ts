import { ENGINE_GENERATED_DIR, stateWarningBlock, VALID_TYPES } from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generateUntracked(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/untracked.rs`,
		`${stateWarningBlock()}

use crate::simulation::*;
use borger_plugin_sdk::traits::UntrackedState;

${VALID_TYPES}

${flattened.output
	.map((group) =>
		group
			.map(function generateConstructor(struct) {
				return `impl UntrackedState for ${struct.name}
{
	fn reset_untracked(&mut self)
	{
		${struct.fields
			.filter(
				({ outerType, typeKind, netVisibility }) =>
					(typeKind === "struct" && outerType !== "Input") ||
					typeKind === "collection" ||
					netVisibility === "untracked",
			)
			.map(function generateSimConstruct({ name, netVisibilityAttribute }) {
				return `${netVisibilityAttribute}
		self.${name}.reset_untracked();`;
			})
			.join("\n\t\t\n\t\t")}
	}
}`;
			})
			.join("\n\n"),
	)
	.join("\n\n")}
`,
	);
}
