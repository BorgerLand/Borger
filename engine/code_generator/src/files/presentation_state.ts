import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	type FlattenedStruct,
	isPrimitive,
} from "@engine/code_generator/Common.ts";

//key is an outerType, value is CloneToPresentationState::PresentationType
const PRESENTATION_TYPE = new Map<string, (innerType: string) => string>([
	["SlotMap", (innerType) => `Vec<(usize32, ${innerType})>`],
]);

export function generatePresentationStateRS(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BASE_GENERATED_DIR}/presentation_state.rs`,
		`${STATE_WARNING}

use crate::simulation_state;
use crate::presentation_state::CloneToPresentationState;

#[allow(unused_imports)]
use crate::presentation_state::ClientState;

${VALID_TYPES}

${simStructs
	.map((group) =>
		group
			.map(function generatePresentationStruct(struct) {
				const presentationStructName =
					struct.name === "SimulationState" ? "PresentationState" : struct.name;

				return `#[derive(Debug)]
pub struct ${presentationStructName}
{
${struct.fields
	.filter((field) => field.isPresentation)
	.map(function ({ name, outerType, fullType, innerType }) {
		//it is technically possible to use this syntax to avoid needing PRESENTATION_TYPE map:
		//<SlotMap<simulation_state::ClientState> as CloneToPresentationState>::PresentationState
		//but sticking simulation_state inside the generics make it difficult
		const presentationType = PRESENTATION_TYPE.get(outerType)?.(innerType) ?? fullType;
		return `	pub ${name}: ${presentationType},`;
	})
	.join("\n")}
}

impl CloneToPresentationState for simulation_state::${struct.name}
{
	type PresentationState = ${presentationStructName};
	
	#[cfg(feature = "client")]
	fn clone_to_presentation(&self) -> Self::PresentationState
	{
		Self::PresentationState
		{
${struct.fields
	.filter((field) => field.isPresentation)
	.map(function ({ name, outerType, netVisibility }) {
		let presentationGetter;
		if (!isPrimitive(outerType) && netVisibility !== "Untracked")
			presentationGetter = ".clone_to_presentation()";
		else presentationGetter = ".clone()";

		return `			${name}: self.${name}${presentationGetter},`;
	})
	.join("\n")}
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
