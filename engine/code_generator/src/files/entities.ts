import { BASE_GENERATED_DIR, STATE_WARNING, type FlattenedStruct } from "@engine/code_generator/Common.ts";

export function generateEntitiesRS(simStructs: FlattenedStruct[][]) {
	const entities = simStructs[0][0].fields
		.filter((field) => field.isEntity)
		.map((entityField) => ({
			field: entityField,
			struct: simStructs.find((group) => group[0].name === entityField.innerType)![0],
		}));

	Bun.write(
		`${BASE_GENERATED_DIR}/entities.rs`,
		`${STATE_WARNING}

use crate::presentation_state::*;
use wasm_bindgen::prelude::*;
use crate::entities::{interpolate_type, Entity, InterpolatedEntityType};
use crate::js_bindings::JSBindings;
use glam::{Vec3A, Quat};

#[derive(Clone, Copy, PartialEq, Eq)]
#[wasm_bindgen]
pub enum EntityKind
{
${entities.map((entity) => `	${entity.struct.name},`).join("\n")}
}

#[derive(Default)]
pub struct EntityBindings
{
${entities.map((entity) => `	pub ${entity.field.name}: InterpolatedEntityType,`).join("\n")}
}

${entities
	.map(
		(entity) => `impl Entity for ${entity.struct.name}
{
	const KIND: EntityKind = EntityKind::${entity.struct.name};
	
	fn get_pos(&self) -> Vec3A
	{
		${entity.struct.fields.some((field) => field.name === "pos") ? "self.pos" : "Vec3A::ZERO"}
	}
	
	fn get_rot(&self) -> Quat
	{
		${entity.struct.fields.some((field) => field.name === "rot") ? "self.rot" : "Quat::IDENTITY"}
	}
	
	fn get_scl(&self) -> Vec3A
	{
		${entity.struct.fields.some((field) => field.name === "scl") ? "self.scl" : "Vec3A::ONE"}
	}
}`,
	)
	.join("\n\n")}

pub fn interpolate
(
	prv_tick: Option<&PresentationTick>,
	cur_tick: &PresentationTick,
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
`,
	);
}
