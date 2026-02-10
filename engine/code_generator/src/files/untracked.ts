import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	type FlattenedStruct,
	isCollection,
} from "@engine/code_generator/common.ts";

export function generateUntracked(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BASE_GENERATED_DIR}/untracked.rs`,
		`${STATE_WARNING}

use crate::simulation_state::*;
use crate::untracked::UntrackedState;

${VALID_TYPES}

${simStructs
	.map((group) =>
		group
			.map(function generateConstructor(struct) {
				return `impl UntrackedState for ${struct.name}
{
	fn reset_untracked(&mut self)
	{
		${struct.fields
			.filter(
				({ outerType, isCustomStruct, netVisibility }) =>
					outerType !== "InputState" &&
					(isCustomStruct || isCollection(outerType) || netVisibility === "Untracked"),
			)
			.map(function generateSimConstruct({ name }) {
				return `self.${name}.reset_untracked();`;
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
