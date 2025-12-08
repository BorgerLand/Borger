import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isPrimitive,
	type FlattenedStruct,
	isUtility,
	isCollection,
} from "@engine/code_generator/Common.ts";

export function generateConstructorsRS(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BASE_GENERATED_DIR}/constructors.rs`,
		`${STATE_WARNING}

use crate::simulation_state::*;
use crate::constructors::{ConstructCustomStruct, ConstructCollectionOrUtilityType};
use crate::ClientStateKind;
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
	fn construct(path: &Rc<Vec<usize32>>, _: ClientStateKind) -> Self
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
					if (outerType === "Vec3A" && struct.isEntity && name === "scl") {
						//let entity scale default to 1, 1, 1
						constructor = "ONE";
					} else if (
						isPrimitive(outerType) ||
						outerType === "InputState" ||
						netVisibility === "Untracked"
					) {
						if (outerType === "InputState") outerType = "InputStateHistory";
						constructor = `default()`;
					} else if (isCollection(outerType) || isUtility(outerType)) {
						constructor = `construct
			(
				path,
				${fieldID},
				
				#[cfg(feature = "server")]
				NetVisibility::${netVisibility}
			)`;
					} else {
						//struct
						constructor = `construct(path, ClientStateKind::${struct.clientKind})`;
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
