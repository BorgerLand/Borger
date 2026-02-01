import {
	BASE_GENERATED_DIR,
	isCollection,
	isPrimitive,
	isUtility,
	STATE_WARNING,
	VALID_TYPES,
	type FlattenedStruct,
} from "@engine/code_generator/Common.ts";
import type { PrimitiveType } from "@engine/code_generator/StateSchema.ts";

export function generateInterpolationRS(simStructs: FlattenedStruct[][]) {
	const entities = simStructs[0][0].fields
		.filter((field) => field.isEntity)
		.map(function (entityField) {
			const group = simStructs.find((group) => group[0].name === entityField.innerType)!;
			return {
				field: entityField,
				group,
				mainStruct: group[0],
			};
		});

	Bun.write(
		`${BASE_GENERATED_DIR}/interpolation.rs`,
		`${STATE_WARNING}

use crate::presentation_state::*;
use wasm_bindgen::prelude::*;
use crate::js_bindings::JSBindings;
use glam::{Vec3, Mat4};
use crate::interpolation::
{
	interpolate_type,
	Interpolate,
	Entity,
	EntityInstanceBindings
};

${VALID_TYPES}

#[derive(Clone, Copy, PartialEq, Eq)]
#[wasm_bindgen]
pub enum EntityKind
{
${entities.map((entity) => `	${entity.mainStruct.name},`).join("\n")}
}

#[derive(Default)]
pub struct EntityBindings
{
${entities.map((entity) => `	pub ${entity.field.name}: Vec<EntityInstanceBindings<${entity.mainStruct.name}>>,`).join("\n")}
}

${entities
	.map(
		(entity) => `impl Entity for ${entity.mainStruct.name}
{
	const KIND: EntityKind = EntityKind::${entity.mainStruct.name};
	
	fn get_matrix_world(&self) -> Mat4
	{
		Mat4::from_scale_rotation_translation
		(
			${entity.mainStruct.fields.some((field) => field.name === "scl") ? "self.scl.into()" : "Vec3::ONE"},
			${entity.mainStruct.fields.some((field) => field.name === "rot") ? "self.rot" : "Quat::IDENTITY"},
			${entity.mainStruct.fields.some((field) => field.name === "pos") ? "self.pos.into()" : "Vec3::ZERO"},
		)
	}
}`,
	)
	.join("\n\n")}

pub fn interpolate_entities
(
	prv_tick: Option<&SimulationOutput>,
	cur_tick: &SimulationOutput,
	received_new_tick: bool,
	amount: f32,
	bindings: &mut JSBindings,
)
{
${entities
	.map(
		(entity) => `	//${entity.field.name}
	interpolate_type
	(
		received_new_tick,
		prv_tick.map(|tick| tick.state.${entity.field.name}.as_slice()).unwrap_or(&[]),
		&cur_tick.state.${entity.field.name},
		&mut bindings.entities.${entity.field.name},
		amount,
		&bindings.cache
	);`,
	)
	.join("\n\t\n")}
}

${entities
	.map((entity) =>
		entity.group
			.map(function generateInterpolator(struct) {
				return `impl Interpolate for ${struct.name}
{
	fn interpolate(prv: &Self, cur: &Self, amount: f32) -> Self
	{
		Self
		{
${struct.fields
	.filter(({ isPresentation, netVisibility }) => isPresentation && netVisibility !== "Private")
	.map(
		({ name, outerType }) =>
			`			${name}: ${isInterpolable(outerType) ? `Interpolate::interpolate(&prv.${name}, &cur.${name}, amount)` : `cur.${name}.clone()`},`,
	)
	.join("\n")}
		}
	}
}`;
			})
			.join("\n\n"),
	)
	.join("\n\n")}

${interpolators
	.map(
		([name, func]) =>
			`impl Interpolate for ${name}
{
	fn interpolate(prv: &Self, cur: &Self, amount: f32) -> Self
	{
		${func}
	}
}`,
	)
	.join("\n\n")}
`,
	);
}

const f64 = "let amount = amount as f64;\n		";
const scalar = "prv * (1.0 - amount) + cur * amount";
const vec = "prv.lerp(*cur, amount)";
const quat = "prv.slerp(*cur, amount)";
const interpolators: [PrimitiveType, string][] = [
	["f32", scalar],
	["f64", f64 + scalar],
	["Vec2", vec],
	["DVec2", f64 + vec],
	["Vec3A", vec],
	["DVec3", f64 + vec],
	["Quat", quat],
	["DQuat", f64 + quat],
];

//should return true for primitives listed in interpolators + custom structs
function isInterpolable(outerType: string) {
	return (
		!isUtility(outerType) &&
		!isCollection(outerType) &&
		(!isPrimitive(outerType) || interpolators.some((e) => e[0] === outerType))
	);
}
