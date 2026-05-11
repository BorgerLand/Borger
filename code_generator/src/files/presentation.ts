import {
	BORGER_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isGeneric,
	isPrimitive,
	isUtility,
	type FlattenedStruct,
} from "@borger/code_generator/common.ts";

export function generatePresentation(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BORGER_GENERATED_DIR}/presentation.rs`,
		`${STATE_WARNING}

use crate::simulation;
use crate::presentation::PresentTick;
use crate::tick::TickID;

use crate::presentation::Client;

${VALID_TYPES}

${simStructs
	.map((group) =>
		group
			.filter(presentationStructFilter)
			.map(function generatePresentationStruct(struct) {
				const presentationStructName = getPresentationStructName(struct.name);

				return `#[allow(non_camel_case_types)]
${presentationStructName === "PresentationOutput" ? "pub" : "pub(crate)"} struct ${presentationStructName}
{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationStructFields({ name, outerType, fullType, innerType }) {
		let presentationType;
		if (isGeneric(outerType))
			presentationType = `<${outerType}<simulation::${innerType}> as PresentTick>::PresentationOutput`;
		else if (isUtility(outerType)) presentationType = `<${outerType} as PresentTick>::PresentationOutput`;
		else presentationType = fullType;

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
	.map(function generatePresentationImpl({ name, outerType }) {
		let presentationGetter;
		if (isPrimitive(outerType)) presentationGetter = "";
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
