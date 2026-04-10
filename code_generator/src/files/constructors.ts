import {
	BORGER_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isPrimitive,
	type FlattenedStruct,
	isUtility,
	isCollection,
	nvEnum,
} from "@borger/code_generator/common.ts";

export function generateConstructors(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BORGER_GENERATED_DIR}/constructors.rs`,
		`${STATE_WARNING}

use crate::simulation_state::*;
use crate::constructors::{ConstructCustomStruct, ConstructCollectionOrUtilityType};
use crate::ClientKind;
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

${VALID_TYPES}

${simStructs
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
				}) {
					let constructor;
					if (isPrimitive(outerType) || outerType === "Input" || netVisibility === "untracked") {
						if (outerType === "Input") outerType = "InputHistory";
						constructor = `default()`;
					} else if (isCollection(outerType) || isUtility(outerType)) {
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
