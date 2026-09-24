import { ENGINE_GENERATED_DIR, stateWarningBlock, VALID_TYPES } from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generateSimulation(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/simulation.rs`,
		`${stateWarningBlock()}

use crate::simulation::{Client, InputHistory};
use std::rc::Rc;

#[cfg(feature = "session_replay")]
use serde::{Deserialize, Serialize};

${VALID_TYPES}

${flattened.input
	.map(function generateInputStruct(struct) {
		return `#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "session_replay", derive(Deserialize, Serialize))]
#[allow(non_camel_case_types)]
pub struct ${struct.name}
{
${struct.fields
	.map(function generateInputStructField({ name, outerType, fieldID }) {
		return `	pub ${name}: ${outerType}, //diff path [${fieldID}]`;
	})
	.join("\n\t\n")}
}`;
	})
	.join("\n\n")}

${flattened.output
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
			outerType,
			innerType,
			netVisibility,
			typeKind,
		}) {
			let fieldVisibilityQualifier; //completely unrelated to netVisibility
			if (typeKind === "primitive" && netVisibility !== "untracked")
				fieldVisibilityQualifier = "pub(crate) ";
			else fieldVisibilityQualifier = "pub "; //structs+collections+plugins

			let fullType;
			if (typeKind === "collection") fullType = `${outerType}<${innerType}>`;
			else if (outerType === "Input") fullType = "InputHistory";
			else fullType = outerType;

			const field = `${fieldVisibilityQualifier}${name}: ${fullType},`;
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
