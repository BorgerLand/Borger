import {
	BORGER_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isPrimitive,
	type AllFlattenedStructs,
} from "@borger/code_generator/common.ts";

export function generateSimulationState(structs: AllFlattenedStructs) {
	Bun.write(
		`${BORGER_GENERATED_DIR}/simulation_state.rs`,
		`${STATE_WARNING}

use crate::simulation_state::{Client, InputHistory};
use std::rc::Rc;

#[cfg(feature = "session_replay")]
use serde::{Deserialize, Serialize};

${VALID_TYPES}

${structs.input
	.map(function generateInputStruct(struct) {
		return `#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "session_replay", derive(Deserialize, Serialize))]
#[allow(non_camel_case_types)]
pub struct ${struct.name}
{
${struct.fields
	.map(function generateInputStructField({ name, fullType, fieldID }) {
		return `	pub ${name}: ${fullType}, //diff path [${fieldID}]`;
	})
	.join("\n\t\n")}
}`;
	})
	.join("\n\n")}

${structs.sim
	.map((group) =>
		group
			.map(function generateSimulationStruct(struct) {
				//primitive fields need setter/getter
				return `#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct ${struct.name}
{
	pub(crate) _diff_path: Rc<Vec<usize32>>,${struct.fields
		.map(function generateSimulationStructField({
			name,
			netVisibilityAttribute,
			fullType,
			outerType,
			netVisibility,
		}) {
			let fieldVisibilityQualifier; //completely unrelated to netVisibility
			if (isPrimitive(outerType) && netVisibility !== "untracked")
				fieldVisibilityQualifier = "pub(crate) ";
			else fieldVisibilityQualifier = "pub "; //structs+collections+utilities

			let actualType;
			if (fullType === "Input") actualType = "InputHistory";
			else actualType = fullType;

			const field = `${fieldVisibilityQualifier}${name}: ${actualType},`;
			return `
	
	${netVisibilityAttribute}
	${field}`;
		})
		.join("")}
}`;
			})
			.join("\n\n"),
	)
	.join("\n\n")}
`,
	);
}
