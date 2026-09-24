import {
	ENGINE_GENERATED_DIR,
	stateWarningBlock,
	VALID_TYPES,
	nvEnum,
} from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generateConstructors(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/constructors.rs`,
		`${stateWarningBlock()}

use crate::simulation::*;
use borger_plugin_sdk::traits::{ConstructCustomStruct, ConstructPlugin};
use borger_plugin_sdk::ClientKind;
use std::rc::Rc;

#[cfg(feature = "server")]
use borger_plugin_sdk::NetVisibility;

${VALID_TYPES}

${flattened.output
	.map((group) =>
		group
			.map(function generateConstructor(struct) {
				return `impl ConstructCustomStruct for ${struct.name}
{
	fn construct(path: &Rc<Vec<usize32>>, _: ClientKind) -> Self
	{
		Self
		{
			_diff_path: path.clone(),${struct.fields
				.map(function generateSimConstruct({
					name,
					netVisibility,
					netVisibilityAttribute,
					outerType,
					fieldID,
					typeKind,
				}) {
					let constructor;
					if (typeKind === "primitive" || outerType === "Input" || netVisibility === "untracked") {
						if (outerType === "Input") outerType = "InputHistory";
						constructor = `default()`;
					} else if (typeKind === "collection" || typeKind === "plugin") {
						constructor = `construct
			(
				path,
				${fieldID},
				
				#[cfg(feature = "server")]
				${nvEnum(netVisibility)}
			)`;
					} else {
						//struct
						constructor = `construct(path, ClientKind::${struct.clientKind})`;
					}

					const field = `${name}: ${outerType}::${constructor},`;
					return `
			
			${netVisibilityAttribute}
			${field}`;
				})
				.join("")}
		}
	}
}`;
			})
			.join("\n\n"),
	)
	.join("\n\n")}
`,
	);
}
