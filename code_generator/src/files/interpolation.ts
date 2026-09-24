import { ENGINE_GENERATED_DIR, stateWarningBlock, VALID_TYPES } from "@borger/code_generator/common.ts";
import { interpolablePrimitiveTypeSchema, type PrimitiveType } from "@borger/code_generator/state_schema.ts";
import {
	getPresentationStructName,
	presentationStructFilter,
} from "@borger/code_generator/files/presentation.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generateInterpolation(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/interpolation.rs`,
		`${stateWarningBlock()}

use crate::simulation;
use crate::presentation;
use borger_plugin_sdk::traits::{PresentTick, Interpolate, InterpolateTicks};

${VALID_TYPES}

${flattened.output
	.map((group) =>
		group
			.filter(presentationStructFilter)
			.map(function generateInterpolationStruct(struct) {
				const interpolationStructName = getInterpolationStructName(struct.name);

				return `#[allow(non_camel_case_types, private_interfaces)]
pub struct ${interpolationStructName}
{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generateInterpolationStructFields({ name, outerType, typeKind, innerType }) {
		//yes i know this is vile
		let interpolationType;
		if (typeKind === "collection")
			interpolationType = `<<${outerType}<simulation::${innerType}> as PresentTick>::PresentationOutput as InterpolateTicks>::InterpolationOutput`;
		else if (typeKind === "plugin")
			interpolationType = `<<${outerType} as PresentTick>::PresentationOutput as InterpolateTicks>::InterpolationOutput`;
		else interpolationType = outerType;

		return `	pub ${name}: ${interpolationType},`;
	})
	.join("\n")}
}

${generateInterpolateTicksImpl(false)}${
					struct.clientKind === "Remote"
						? `

${generateInterpolateTicksImpl(true)}`
						: ""
				}`;

				function generateInterpolateTicksImpl(downgradeScope: boolean) {
					const presentationStructName = getPresentationStructName(struct.name);
					const downgradedName = presentationStructName.replace(/Remote$/, "Owned"); //only valid if downgradeScope true
					return `impl InterpolateTicks${downgradeScope ? `<presentation::${downgradedName}>` : ""} for presentation::${presentationStructName}
{
	type InterpolationOutput = ${interpolationStructName};
	fn interpolate_and_diff(_prv: Option<&${downgradeScope ? `presentation::${downgradedName}` : "Self"}>, _cur: &Self, _amount: f32, _received_new_tick: bool) -> Self::InterpolationOutput
	{
		Self::InterpolationOutput
		{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generateInterpolationImpl({ name, outerType, presentation, typeKind }) {
		let interpolationGetter;
		if (typeKind !== "primitive")
			interpolationGetter = `InterpolateTicks::interpolate_and_diff
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
			interpolationGetter = `_cur.${name}`;
		else
			interpolationGetter = `if let Some(prv) = _prv
			{
				${outerType}::interpolate(prv.${name}, _cur.${name}, _amount)
			}
			else
			{
				_cur.${name}
			}`;

		return `			${name}: ${interpolationGetter},`;
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

function getInterpolationStructName(simStructName: string) {
	return simStructName === "State" ? "InterpolationOutput" : simStructName;
}
