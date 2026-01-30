import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	type FlattenedStruct,
	isPrimitive,
} from "@engine/code_generator/Common.ts";
import { simplePrimitives, type multiFieldPrimitives } from "@engine/code_generator/StateSchema.ts";

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
use wasm_bindgen::prelude::*;

#[allow(unused_imports)]
use crate::presentation_state::ClientState;

#[cfg(feature = "client")]
use
{
	crate::presentation_state::get_entity_from_jsdata,
	glam::Mat4,
};

${VALID_TYPES}

${simStructs
	.map((group) =>
		group
			.map(function generatePresentationStruct(struct) {
				const presentationStructName =
					struct.name === "SimulationState" ? "PresentationState" : struct.name;

				return `#[derive(Debug)]
#[allow(non_camel_case_types)]
#[wasm_bindgen]
pub struct ${presentationStructName}
{
${struct.fields
	.filter((field) => field.isPresentation)
	.map(function ({ name, outerType, fullType, innerType }) {
		//it is technically possible to use this syntax to avoid needing PRESENTATION_TYPE map:
		//<SlotMap<simulation_state::ClientState> as CloneToPresentationState>::PresentationState
		//but sticking simulation_state inside the generics make it difficult
		const presentationType = PRESENTATION_TYPE.get(outerType)?.(innerType) ?? fullType;
		return `	#[wasm_bindgen(skip)]
	pub ${name}: ${presentationType},`;
	})
	.join("\n\n")}
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
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl ${presentationStructName}
{
${struct.fields
	.filter(
		({ netVisibility, isPresentation, outerType }) =>
			struct.isEntity && isPresentation && netVisibility !== "Private" && isPrimitive(outerType),
	)
	.map(function ({ name, fullType }) {
		if ((simplePrimitives as readonly string[]).includes(fullType)) {
			return generateJSReader(presentationStructName, name, fullType);
		} else {
			switch (fullType as (typeof multiFieldPrimitives)[number]) {
				case "Vec2":
					return generateMultiFieldReader(presentationStructName, name, "xy", "f32");
				case "DVec2":
					return generateMultiFieldReader(presentationStructName, name, "xy", "f64");
				case "Vec3A":
					return generateMultiFieldReader(presentationStructName, name, "xyz", "f32");
				case "DVec3":
					return generateMultiFieldReader(presentationStructName, name, "xyz", "f64");
				case "Quat":
					return generateMultiFieldReader(presentationStructName, name, "xyzw", "f32");
				case "DQuat":
					return generateMultiFieldReader(presentationStructName, name, "xyzw", "f64");
			}
		}
	})
	.join("\n\n")}
}`;
			})
			.join("\n\n"),
	)
	.join("\n\n")}
`,
	);
}

//note these js reader functions are extremely slow due to
//ffi overhead (not to mention clunky+unsafe to work with).
//it would be 129x faster to simply use a DataView, however,
//wasm bindgen currently doesn't support constants. ideally
//would be able to use const exports to do something like this:
//#[wasm_bindgen]
//pub const CHARACTER_POS_X = mem::offset_of!(Character, x);
//calling the wasm getter a billion times = 55 seconds
//view.getFloat32(ptr + 4, true) x1billion = 0.43 seconds!
//technically there are workarounds involving more manual
//codegen, however, i'm going with the simplest possible
//solution for now
function generateJSReader(structName: string, fieldName: string, fullType: string, subfield?: string) {
	return `	pub unsafe fn get_${fieldName}${subfield ? `_${subfield}` : ``}(rs_ptr: *const Mat4) -> ${fullType}
	{
		let entity = unsafe { get_entity_from_jsdata::<${structName}>(rs_ptr) };
		entity.${fieldName}${subfield ? `.${subfield}` : ``}
	}`;
}

function generateMultiFieldReader(
	structName: string,
	fieldName: string,
	components: string,
	subfieldType: string,
) {
	return Array.from(components, (component) =>
		generateJSReader(structName, fieldName, subfieldType, component),
	).join("\n\n");
}
