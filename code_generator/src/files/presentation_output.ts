import {
	ENGINE_GENERATED_DIR,
	presentationStructFilter,
	stateWarningBlock,
	VALID_TYPES,
} from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";
import { interpolablePrimitiveTypeSchema, type PrimitiveType } from "@borger/code_generator/state_schema.ts";
import { getPresentationCollectStructName } from "@borger/code_generator/files/presentation_collect.ts";

export function generatePresentationOutput(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/presentation_output.rs`,
		`${stateWarningBlock()}

use crate::simulation;
use crate::presentation_collect;
use borger_plugin_sdk::traits::{PresentationCollect, Interpolate, PresentationOutput};

${VALID_TYPES}

${flattened.output
	.map((group) =>
		group
			.filter(presentationStructFilter)
			.map(function generatePresentationOutputStruct(struct) {
				const presentationOutputStructName = getPresentationOutputStructName(struct.name);

				return `#[allow(non_camel_case_types, private_interfaces)]
pub struct ${presentationOutputStructName}
{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationOutputFields({ name, outerType, typeKind, innerType }) {
		//yes i know this is vile
		let presentationOutputType;
		if (typeKind === "collection")
			presentationOutputType = `<<${outerType}<simulation::${innerType}> as PresentationCollect>::PresentationCollect as PresentationOutput>::PresentationOutput`;
		else if (typeKind === "plugin")
			presentationOutputType = `<<${outerType} as PresentationCollect>::PresentationCollect as PresentationOutput>::PresentationOutput`;
		else presentationOutputType = outerType;

		return `	pub ${name}: ${presentationOutputType},`;
	})
	.join("\n")}
}

${generatePresentationOutputStructImpl(false)}${
					struct.clientKind === "Remote"
						? `

${generatePresentationOutputStructImpl(true)}`
						: ""
				}`;

				function generatePresentationOutputStructImpl(downgradeScope: boolean) {
					const presentationCollectStructName = getPresentationCollectStructName(struct.name);
					const downgradedName = presentationCollectStructName.replace(/Remote$/, "Owned"); //only valid if downgradeScope true
					return `impl PresentationOutput${downgradeScope ? `<presentation_collect::${downgradedName}>` : ""} for presentation_collect::${presentationCollectStructName}
{
	type PresentationOutput = ${presentationOutputStructName};
	fn presentation_output(_prv: Option<&${downgradeScope ? `presentation_collect::${downgradedName}` : "Self"}>, _cur: &Self, _amount: f32, _received_new_tick: bool) -> Self::PresentationOutput
	{
		Self::PresentationOutput
		{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generatePresentationOutputFieldImpl({ name, outerType, presentation, typeKind }) {
		let outputter;
		if (typeKind !== "primitive")
			outputter = `PresentationOutput::presentation_output
			(
				_prv.map(|prv| &prv.${name}),
				&_cur.${name},
				_amount,
				_received_new_tick
			)`;
		else if (
			presentation === "clone" ||
			!(interpolablePrimitiveTypeSchema.options as PrimitiveType[]).includes(outerType as PrimitiveType)
		)
			outputter = `_cur.${name}`;
		else
			outputter = `if let Some(prv) = _prv
			{
				${outerType}::interpolate(prv.${name}, _cur.${name}, _amount)
			}
			else
			{
				_cur.${name}
			}`;

		return `			${name}: ${outputter},`;
	})
	.join("\n\n")}
		}
	}
}`;
				}
			})
			.join("\n\n"),
	)
	.join("\n\n")}
`,
	);
}

function getPresentationOutputStructName(simStructName: string) {
	return simStructName === "State" ? "PresentationOutputState" : simStructName;
}
