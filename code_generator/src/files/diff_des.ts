import {
	ENGINE_GENERATED_DIR,
	stateWarningBlock,
	getNestedPath,
	VALID_TYPES,
} from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

//the way the generated file generally works is:
//given a deserialized diff path and value, write
//the value to the main state object. this file
//uses match statements to route the value where
//it needs to go
export function generateDiffDes(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/diff_des.rs`,
		`${stateWarningBlock()}

use crate::simulation::*;
use borger_plugin_sdk::primitive::{PrimitiveSerDes, DeserializeOopsy};
use borger_plugin_sdk::traits::{DiffDeserializeCustomStruct, DiffDeserializePlugin};

#[cfg(feature = "server")]
use crate::simulation::Input;

#[cfg(feature = "client")]
use
{
	borger_plugin_sdk::diff_ser::DiffSerializer,
	borger_plugin_sdk::multiplayer_tradeoff::Impl,
	std::vec,
};

${VALID_TYPES}

#[cfg(feature = "server")]
pub fn des_rx_input(input: &mut Input, mut ser_rx_buffer: impl ExactSizeIterator<Item = u8>) -> Result<(), DeserializeOopsy>
{
	let buffer = &mut ser_rx_buffer;
	while buffer.len() > 0
	{
		let field_id = usize32::des_rx(buffer)?;
		match field_id
		{
${flattened.input
	.map((struct) =>
		struct.fields
			.filter(
				({ netVisibility, typeKind }) => typeKind === "primitive" && netVisibility !== "untracked",
			)
			.map(function generateStructField({ name, fieldID, outerType }) {
				const fieldPath = getNestedPath(flattened.input[0].path, struct.path, name);

				return `			${fieldID} => input.${fieldPath} = ${outerType}::des_rx(buffer)?,`;
			})
			.join("\n\t\n"),
	)
	.join("\n\t\n")}
			
			_ => return Err(DeserializeOopsy),
		}
	}
	
	Ok(())
}

${flattened.output
	.map(function generateDeserializeState(group) {
		const rootStruct = group[0];
		return `impl DiffDeserializeCustomStruct for ${rootStruct.name}
{
	fn set_field_rollback(&mut self, field_id: usize32, _buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>
	{
		match field_id
		{
${group
	.map((struct) =>
		struct.fields
			.filter(
				({ netVisibility, typeKind }) => typeKind === "primitive" && netVisibility !== "untracked",
			)
			.map(function generateSetFieldRollback({ name, netVisibilityAttribute, fieldID }) {
				const field = getNestedPath(rootStruct.path, struct.path, name);

				return `			${netVisibilityAttribute}
			${fieldID} => self.${field} = PrimitiveSerDes::des_rollback(_buffer)?,`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
			
			_ => return Err(DeserializeOopsy),
		};
		
		#[allow(unreachable_code)]
		Ok(())
	}
	
	#[cfg(feature = "client")]
	fn set_field_rx(&mut self, field_id: usize32, _buffer: &mut vec::IntoIter<u8>, _diff: &mut DiffSerializer<Impl>) -> Result<(), DeserializeOopsy>
	{
		match field_id
		{
${group
	.map((struct) =>
		struct.fields
			.filter(
				({ netVisibility, typeKind }) =>
					typeKind === "primitive" && netVisibility !== "private" && netVisibility !== "untracked",
			)
			.map(function generateSetFieldRx({ name, netVisibilityAttribute, fieldID, outerType }) {
				const field = getNestedPath(rootStruct.path, struct.path, `set_${name}`);

				return `			${netVisibilityAttribute}
			${fieldID} => { self.${field}(${outerType}::des_rx(_buffer)?, _diff); },`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
			
			_ => return Err(DeserializeOopsy),
		};
		
		#[allow(unreachable_code)]
		Ok(())
	}
	
	fn get_plugin(&mut self, field_id: usize32) -> Result<&mut dyn DiffDeserializePlugin, DeserializeOopsy>
	{
		match field_id
		{
${group
	.map((struct) =>
		struct.fields
			.filter(
				({ typeKind, netVisibility }) =>
					(typeKind === "collection" || typeKind === "plugin") && netVisibility !== "untracked",
			)
			.map(function generateGetter({ name, netVisibilityAttribute, fieldID }) {
				const field = getNestedPath(rootStruct.path, struct.path, name);

				return `			${netVisibilityAttribute}
			${fieldID} => Ok(&mut self.${field}),`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
			
			_ => return Err(DeserializeOopsy),
		}
	}
}`;
	})
	.join("\n\n")}
`,
	);
}
