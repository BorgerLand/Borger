import { ENGINE_GENERATED_DIR, stateWarningBlock, VALID_TYPES } from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput, FlattenedStruct } from "@borger/code_generator/flatten.ts";

export function generatePresentation(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/presentation.rs`,
		`${stateWarningBlock()}

use crate::simulation;
use borger_plugin_sdk::TickID;
use borger_plugin_sdk::traits::PresentTick;

${VALID_TYPES}

${flattened.output
	.map((group) =>
		group
			.filter(presentationStructFilter)
			.map(function generatePresentationStruct(struct) {
				const presentationStructName = getPresentationStructName(struct.name);

				return `#[allow(non_camel_case_types)]
pub struct ${presentationStructName}
{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationStructFields({ name, outerType, innerType, typeKind }) {
		let presentationType;
		if (typeKind === "collection")
			presentationType = `<${outerType}<simulation::${innerType}> as PresentTick>::PresentationOutput`;
		else if (typeKind === "plugin")
			presentationType = `<${outerType} as PresentTick>::PresentationOutput`;
		else presentationType = outerType;

		return `	pub(crate) ${name}: ${presentationType},`;
	})
	.join("\n\n")}
}

impl PresentTick for simulation::${struct.name}
{
	type PresentationOutput = ${presentationStructName};
	fn clone_to_presentation(&self, _tick: TickID) -> Self::PresentationOutput
	{
		Self::PresentationOutput
		{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationImpl({ name, typeKind }) {
		let presentationGetter;
		if (typeKind === "primitive") presentationGetter = "";
		else presentationGetter = ".clone_to_presentation(_tick)";

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

export function getPresentationStructName(simStructName: string) {
	return simStructName === "State" ? "PresentationOutput" : simStructName;
}

export function presentationStructFilter(struct: FlattenedStruct) {
	return !(
		(struct.clientKind === "Remote" && struct.netVisibility !== "public") ||
		struct.netVisibility === "private"
	);
}
