import {
	BORGER_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isGeneric,
	isPrimitive,
	isUtility,
	type FlattenedStruct,
} from "@borger/code_generator/common.ts";
import {
	interpolablePrimitiveTypeSchema,
	type InterpolablePrimitiveType,
	type PrimitiveType,
} from "@borger/code_generator/state_schema.ts";
import {
	getPresentationStructName,
	presentationStructFilter,
} from "@borger/code_generator/files/presentation.ts";

export function generateInterpolation(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BORGER_GENERATED_DIR}/interpolation.rs`,
		`${STATE_WARNING}

use crate::interpolation::Interpolate;

#[cfg(feature = "client")]
use
{
	crate::simulation_state,
	crate::presentation::{self, PresentTick},
	crate::interpolation::InterpolateTicks,
};

#[cfg(feature = "client")]
#[allow(unused_imports)]
use crate::presentation::Client;

${VALID_TYPES}

${simStructs
	.map((group) =>
		group
			.filter(presentationStructFilter)
			.map(function generateInterpolationStruct(struct) {
				const interpolationStructName = getInterpolationStructName(struct.name);

				return `#[cfg(feature = "client")]
#[allow(non_camel_case_types, private_interfaces)]
pub struct ${interpolationStructName}
{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generateInterpolationStructFields({ name, outerType, fullType, innerType }) {
		let interpolationType;
		if (isGeneric(outerType))
			//yes i know this is vile
			interpolationType = `<<${outerType}<simulation_state::${innerType}> as PresentTick>::PresentationState as InterpolateTicks>::InterpolationState`;
		else if (isUtility(outerType))
			interpolationType = `<<${outerType} as PresentTick>::PresentationState as InterpolateTicks>::InterpolationState`;
		else interpolationType = fullType;

		return `	pub ${name}: ${interpolationType},`;
	})
	.join("\n")}
}

#[cfg(feature = "client")]
impl InterpolateTicks for presentation::${getPresentationStructName(struct.name)}
{
	type InterpolationState = ${interpolationStructName};
	fn interpolate_and_diff(_prv: Option<&Self>, _cur: &Self, _amount: f32, _received_new_tick: bool) -> Self::InterpolationState
	{
		Self::InterpolationState
		{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generateInterpolationImpl({ name, outerType, presentation }) {
		let interpolationGetter;
		if (!isPrimitive(outerType))
			interpolationGetter = `InterpolateTicks::interpolate_and_diff
			(
				_prv.map(|prv| &prv.${name}),
				&_cur.${name},
				_amount,
				_received_new_tick
			)`;
		else if (
			presentation === "clone" ||
			!(interpolablePrimitiveTypeSchema.options as PrimitiveType[]).includes(outerType)
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
			})
			.join("\n\n"),
	)
	.join("\n\n")}

${interpolablePrimitiveTypeSchema.options
	.map(
		(name) =>
			`impl Interpolate for ${name}
{
	fn interpolate(prv: Self, cur: Self, amount: f32) -> Self
	{
		${interpolatePrimitive(name)}
	}
}`,
	)
	.join("\n\n")}
`,
	);
}

const f64 = "let amount = amount as f64;\n\t\t";
const scalar = "prv * (1.0 - amount) + cur * amount";
const vec = "prv.lerp(cur, amount)";
const quat = "prv.slerp(cur, amount)";
function interpolatePrimitive(type: InterpolablePrimitiveType): string {
	switch (type) {
		case "f32":
			return scalar;
		case "f64":
			return f64 + scalar;
		case "Vec2":
			return vec;
		case "DVec2":
			return f64 + vec;
		case "Vec3":
			return vec;
		case "DVec3":
			return f64 + vec;
		case "Quat":
			return quat;
		case "DQuat":
			return f64 + quat;
	}
}

function getInterpolationStructName(simStructName: string) {
	return simStructName === "SimulationState" ? "InterpolationState" : simStructName;
}
