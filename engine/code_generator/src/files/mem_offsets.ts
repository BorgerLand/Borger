import {
	CLIENT_RS_GENERATED_DIR,
	STATE_WARNING,
	getNestedPath,
	type AllFlattenedStructs,
} from "@borger/code_generator/common.ts";
import { getOutputStructName } from "@borger/code_generator/files/mem_wrappers.ts";
import { presentationStructFilter } from "@borger/code_generator/files/presentation.ts";

export function generateMemOffsets(structs: AllFlattenedStructs) {
	const ioStructs = structs.sim.slice().reverse();
	ioStructs.unshift(structs.input);

	const slotMapInnerTypes: string[] = [];

	Bun.write(
		`${CLIENT_RS_GENERATED_DIR}/mem_offsets.rs`,
		`${STATE_WARNING}

use borger::simulation_state::Input;
use borger::interpolation::*;
use wasm_bindgen::prelude::*;
use js_sys::{Object, Reflect, Number};
use std::mem::offset_of;
use borger::networked_types::collections::slotmap::InterpolationSlotMap;
use borger::networked_types::primitive::usize32;

type Output = InterpolationOutput;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn get_mem_offsets() -> JsValue
{
${ioStructs
	.map(function generateStructGroups(group) {
		const rootStruct = group[0];
		const rootStructName = getOutputStructName(rootStruct.name);

		return `${group
			.filter((struct) => presentationStructFilter(struct) || rootStruct.name === "Input")
			.reverse()
			.map(function generateStructs(struct) {
				const outputStructPath = struct.path.slice();
				if (rootStructName === "Output") outputStructPath.splice(1, 0, "state");

				return `	let struct_${struct.name} = Object::new();
${struct.fields
	.filter((field) => field.presentation || rootStruct.name === "Input")
	.map(
		({ name, fullType, isCustomStruct }) =>
			`	Reflect::set
	(
		&struct_${struct.name},
		&"${name}".into(),
		${(function generateFieldOffsets() {
			if (isCustomStruct) {
				return `&struct_${fullType}`;
			} else {
				return `&Number::from(offset_of!(${rootStructName}, ${getNestedPath(rootStruct.path, outputStructPath, name)}) as f64)`;
			}
		})()}
	).unwrap();`,
	)
	.join("\n\t\n")}`;
			})
			.join("\n\t\n")}`;
	})
	.join("\n\t\n")}
	
	let struct_Client = Object::new();
	Reflect::set
	(
		&struct_Client,
		&"owned".into(),
		&Number::from(align_of::<ClientOwned>().max(1) as f64)
	).unwrap();
	
	Reflect::set
	(
		&struct_Client,
		&"remote".into(),
		&Number::from(align_of::<ClientRemote>().max(1) as f64)
	).unwrap();
	
	let struct_Output = Object::new();
	Reflect::set
	(
		&struct_Output,
		&"state".into(),
		&struct_SimulationState
	).unwrap();
	
	Reflect::set
	(
		&struct_Output,
		&"local_client_id".into(),
		&Number::from(offset_of!(Output, local_client_id) as f64)
	).unwrap();
	
	let structs = Object::new();
	Reflect::set(&structs, &"Input".into(), &struct_Input).unwrap();
${structs.sim
	.map(function generateStructs(group) {
		const rootStructName = getOutputStructName(group[0].name);
		return `	Reflect::set(&structs, &"${rootStructName}".into(), &struct_${rootStructName}).unwrap();`;
	})
	.join("\n")}
	Reflect::set(&structs, &"Client".into(), &struct_Client).unwrap();
	
	let slotmap = Object::new();${structs.sim
		.map((group) =>
			group
				.filter(presentationStructFilter)
				.map((struct) =>
					struct.fields
						.filter(({ outerType, presentation }) => outerType === "SlotMap" && presentation)
						.map(function ({ innerType }) {
							slotMapInnerTypes.push(innerType);
							return `
	
	let slotmap_${innerType} = Object::new();
	Reflect::set(&slotmap, &"${innerType}".into(), &slotmap_${innerType}).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"slot_0".into(),
		&Number::from(offset_of!((usize32, ${innerType}), 0) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"slot_1".into(),
		&Number::from(offset_of!((usize32, ${innerType}), 1) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"slots_stride".into(),
		&Number::from(size_of::<(usize32, ${innerType})>() as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"slots_ptr".into(),
		&Number::from(offset_of!(InterpolationSlotMap<${innerType}>, slots_ptr) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"slots_len".into(),
		&Number::from(offset_of!(InterpolationSlotMap<${innerType}>, slots_len) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"removed_ptr".into(),
		&Number::from(offset_of!(InterpolationSlotMap<${innerType}>, removed_ptr) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"removed_len".into(),
		&Number::from(offset_of!(InterpolationSlotMap<${innerType}>, removed_len) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"added_ptr".into(),
		&Number::from(offset_of!(InterpolationSlotMap<${innerType}>, added_ptr) as f64)
	).unwrap();
	Reflect::set
	(
		&slotmap_${innerType},
		&"added_len".into(),
		&Number::from(offset_of!(InterpolationSlotMap<${innerType}>, added_len) as f64)
	).unwrap();`;
						})
						.join(""),
				)
				.join(""),
		)
		.join("")}
	
	let output = Object::new();
	Reflect::set(&output, &"struct".into(), &structs).unwrap();
	Reflect::set(&output, &"slotmap".into(), &slotmap).unwrap();
	output.into()
}

${slotMapInnerTypes
	.map(
		(innerType) => `#[wasm_bindgen]
#[allow(non_snake_case)]
pub unsafe fn slotmap_get_${innerType}(ptr: *const InterpolationSlotMap<${innerType}>, id: usize32) -> Option<*const ${innerType}>
{
	let slotmap = unsafe { &*ptr };
	slotmap.data.get(id).map(|element| element as *const ${innerType})
}`,
	)
	.join("\n\n")}
`,
	);
}
