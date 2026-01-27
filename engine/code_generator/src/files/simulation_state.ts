import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isPrimitive,
	type AllFlattenedStructs,
} from "@engine/code_generator/Common.ts";

export function generateSimulationStateRS(structs: AllFlattenedStructs) {
	Bun.write(
		`${BASE_GENERATED_DIR}/simulation_state.rs`,
		`${STATE_WARNING}

use crate::simulation_state::{ClientState, InputStateHistory};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

${VALID_TYPES}

${structs.input
	.map(function generateInputStruct(struct) {
		return `#[derive(Debug, Default, Clone)]
#[allow(non_camel_case_types)]${
			struct.name === "InputState"
				? `
#[wasm_bindgen]`
				: ""
		}
pub struct ${struct.name}
{
${struct.fields
	.map(function generateInputStructField({ name, fullType, fieldID }) {
		return `${
			struct.name === "InputState"
				? `	#[wasm_bindgen(skip)]
`
				: ""
		}	pub ${name}: ${fullType}, //diff path [${fieldID}]`;
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
			if (isPrimitive(outerType) && netVisibility !== "Untracked")
				fieldVisibilityQualifier = "pub(crate) ";
			else fieldVisibilityQualifier = "pub "; //structs+collections+utilities

			let actualType;
			if (fullType === "InputState") actualType = "InputStateHistory";
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
