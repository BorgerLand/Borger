import {
	ENGINE_GENERATED_DIR,
	presentationStructFilter,
	stateWarningBlock,
	VALID_TYPES,
} from "@borger/code_generator/common.ts";
import fs from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generatePresentationCollect(flattened: FlattenedOutput) {
	fs.writeFileSync(
		`${ENGINE_GENERATED_DIR}/presentation_collect.rs`,
		`${stateWarningBlock()}

use crate::simulation;
use borger_plugin_sdk::TickID;
use borger_plugin_sdk::traits::PresentationCollect;

${VALID_TYPES}

${flattened.output
	.map((group) =>
		group
			.filter(presentationStructFilter)
			.map(function generatePresentationCollectStruct(struct) {
				const presentationCollectStructName = getPresentationCollectStructName(struct.name);

				return `#[allow(non_camel_case_types)]
pub struct ${presentationCollectStructName}
{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationCollectFields({ name, outerType, innerType, typeKind }) {
		let presentationCollectType;
		if (typeKind === "collection")
			presentationCollectType = `<${outerType}<simulation::${innerType}> as PresentationCollect>::PresentationCollect`;
		else if (typeKind === "plugin")
			presentationCollectType = `<${outerType} as PresentationCollect>::PresentationCollect`;
		else presentationCollectType = outerType;

		return `	pub(crate) ${name}: ${presentationCollectType},`;
	})
	.join("\n\n")}
}

impl PresentationCollect for simulation::${struct.name}
{
	type PresentationCollect = ${presentationCollectStructName};
	fn clone_to_presentation(&self, _tick: TickID) -> Self::PresentationCollect
	{
		Self::PresentationCollect
		{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationCollectFieldImpl({ name, typeKind }) {
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

export function getPresentationCollectStructName(simStructName: string) {
	return simStructName === "State" ? "PresentationCollectState" : simStructName;
}
