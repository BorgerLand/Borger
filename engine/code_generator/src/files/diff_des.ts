import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	isPrimitive,
	getFullFieldPath,
	type AllFlattenedStructs,
	VALID_TYPES,
	type FlattenedField,
} from "@engine/code_generator/Common.ts";

//the way the generated file generally works is:
//given a deserialized diff path and value, write
//the value to the main simulation state object.
//this file uses match statements to route the
//value where it needs to go
export function generateDiffDesRS(structs: AllFlattenedStructs) {
	Bun.write(
		`${BASE_GENERATED_DIR}/diff_des.rs`,
		`${STATE_WARNING}

use crate::simulation_state::*;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::diff_des::DiffDeserializeState;
use crate::DeserializeOopsy;
use crate::networked_types::collections::slotmap::SlotMapDynCompat;
use std::collections::VecDeque;

#[cfg(feature = "server")]
use crate::simulation_state::InputState;

#[cfg(feature = "client")]
use
{
	crate::diff_ser::DiffSerializer,
	crate::context::Impl,
};

${VALID_TYPES}

#[cfg(feature = "server")]
pub fn des_rx_input(input: &mut InputState, mut ser_rx_buffer: VecDeque<u8>) -> Result<(), DeserializeOopsy>
{
	let buffer = &mut ser_rx_buffer;
	while buffer.len() > 0
	{
		let field_id = usize32::des_rx(buffer)?;
		match field_id
		{
${structs.input
	.map((struct) =>
		struct.fields
			.filter(({ outerType, netVisibility }) => isPrimitive(outerType) && netVisibility !== "Untracked")
			.map(function generateStructField({ name, fieldID, fullType }) {
				const fieldPath = getFullFieldPath(structs.input[0].path, struct.path, name);

				return `			${fieldID} => input.${fieldPath} = ${fullType}::des_rx(buffer)?,`;
			})
			.join("\n\t\n"),
	)
	.join("\n\t\n")}
			
			_ => return Err(DeserializeOopsy::FieldNotFound),
		}
	}
	
	Ok(())
}

${structs.sim
	.map(function generateDeserializeState(group) {
		const rootStruct = group[0];
		return `impl DiffDeserializeState for ${rootStruct.name}
{
	fn set_field_rollback(&mut self, field_id: usize32, _buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>
	{
		match field_id
		{
${group
	.map((struct) =>
		struct.fields
			.filter(({ outerType, netVisibility }) => isPrimitive(outerType) && netVisibility !== "Untracked")
			.map(function generateSetFieldRollback({ name, netVisibilityAttribute, fieldID }) {
				const field = getFullFieldPath(rootStruct.path, struct.path, name);

				return `			${netVisibilityAttribute}
			${fieldID} => self.${field} = PrimitiveSerDes::des_rollback(_buffer)?,`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
			
			_ => return Err(DeserializeOopsy::FieldNotFound),
		};
		
		#[allow(unreachable_code)]
		Ok(())
	}
	
	#[cfg(feature = "client")]
	fn set_field_rx(&mut self, field_id: usize32, _buffer: &mut VecDeque<u8>, _diff: &mut DiffSerializer<Impl>) -> Result<(), DeserializeOopsy>
	{
		match field_id
		{
${group
	.map((struct) =>
		struct.fields
			.filter(
				({ outerType, netVisibility }) =>
					isPrimitive(outerType) && netVisibility !== "Private" && netVisibility !== "Untracked",
			)
			.map(function generateSetFieldRx({ name, netVisibilityAttribute, fieldID, outerType }) {
				const field = getFullFieldPath(rootStruct.path, struct.path, `set_${name}`);

				return `			${netVisibilityAttribute}
			${fieldID} => { self.${field}(${outerType}::des_rx(_buffer)?, _diff.to_impl()); },`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
			
			_ => return Err(DeserializeOopsy::FieldNotFound),
		};
		
		#[allow(unreachable_code)]
		Ok(())
	}
	
	${generateGetCollectionOrUtility("get_slotmap", "dyn SlotMapDynCompat", (field) => field.outerType === "SlotMap")}
}`;

		//need to call this for every collection and utility type
		function generateGetCollectionOrUtility(
			getterName: string,
			returnType: string,
			structFilter: (field: FlattenedField) => boolean,
		) {
			return `fn ${getterName}(&mut self, field_id: usize32) -> Result<&mut ${returnType}, DeserializeOopsy>
	{
		match field_id
		{
${group
	.map((struct) =>
		struct.fields
			.filter(structFilter)
			.map(function generateGetter({ name, netVisibilityAttribute, fieldID }) {
				const field = getFullFieldPath(rootStruct.path, struct.path, name);

				return `			${netVisibilityAttribute}
			${fieldID} => Ok(&mut self.${field}),`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
			
			_ => return Err(DeserializeOopsy::PathNotFound),
		}
	}`;
		}
	})
	.join("\n\n")}
`,
	);
}
