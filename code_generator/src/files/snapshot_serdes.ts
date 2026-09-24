import { ENGINE_GENERATED_DIR, stateWarningBlock, getNestedPath } from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedField, FlattenedOutput } from "@borger/code_generator/flatten.ts";

//new client: all public data should be serialized
//predict remove: all locally accessible data should be serialized. "all" has different meanings depending on server/client
export function generateSnapshotSerDes(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/snapshot_serdes.rs`,
		`${stateWarningBlock()}

use crate::simulation::*;
use borger_plugin_sdk::primitive::{PrimitiveSerDes, DeserializeOopsy, usize32};
use borger_plugin_sdk::traits::SnapshotState;

${flattened.output
	.map(function generatePredictRemoveImpl(group) {
		const rootStruct = group[0];
		return `impl SnapshotState for ${rootStruct.name}
{
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, _client_id: usize32, _buffer: &mut Vec<u8>)
	{
${group
	.map((struct) =>
		struct.fields
			.filter((field) => rootStruct.clientKind !== "Remote" && canSnapshotNewClient(field))
			.map(function generateSerializeRemoveField({
				name,
				netVisibility,
				netVisibilityAttribute,
				typeKind,
			}) {
				const isClientData = struct.path[1] === "clients";
				const field = getNestedPath(rootStruct.path, struct.path, name);

				let serializer;
				if (typeKind === "primitive") serializer = `self.${field}.ser_tx(_buffer)`;
				else serializer = `self.${field}.ser_tx_new_client(_client_id, _buffer)`; //collections+plugins

				if (isClientData && netVisibility === "owner") {
					//scope filtering: skip sending this state to any
					//client who doesn't need to know about this change
					return `		if self._diff_path[1] == _client_id
		{
			${netVisibilityAttribute}
			${serializer};
		}`;
				} else {
					return `		${netVisibilityAttribute}
		${serializer};`;
				}
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
	}
	
	#[cfg(feature = "client")]
	fn des_rx_new_client(&mut self, _client_id: usize32, _buffer: &mut impl Iterator<Item = u8>) -> Result<(), DeserializeOopsy>
	{
${group
	.map((struct) =>
		struct.fields
			.filter(canSnapshotNewClient)
			.map(function generateSerializeRemoveField({ name, netVisibilityAttribute, typeKind }) {
				const field = getNestedPath(rootStruct.path, struct.path, name);

				let serializer;
				if (typeKind === "primitive")
					serializer = `self.${field} = PrimitiveSerDes::des_rx(_buffer)?`;
				else serializer = `self.${field}.des_rx_new_client(_client_id, _buffer)?`; //collections+plugins

				return `		${netVisibilityAttribute}
		${serializer};`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
		
		Ok(())
	}
	
	fn ser_rollback_predict_remove(&self, _buffer: &mut Vec<u8>)
	{
${group
	.slice()
	.reverse()
	.map((struct) =>
		struct.fields
			.filter(canSnapshotPredictRemove)
			.slice()
			.reverse()
			.map(function generateSerializeRemoveField({ name, netVisibilityAttribute, typeKind }) {
				const field = getNestedPath(rootStruct.path, struct.path, name);

				let serializer;
				if (typeKind === "primitive") serializer = `self.${field}.ser_rollback(_buffer)`;
				else serializer = `self.${field}.ser_rollback_predict_remove(_buffer)`; //collections+plugins

				return `		${netVisibilityAttribute}
		${serializer};`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
	}
	
	fn des_rollback_predict_remove(&mut self, _buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>
	{
${group
	.map((struct) =>
		struct.fields
			.filter(canSnapshotPredictRemove)
			.map(function generateSerializeRemoveField({
				name,
				netVisibility,
				netVisibilityAttribute,
				typeKind,
			}) {
				const field = getNestedPath(rootStruct.path, struct.path, name);

				let serializer;
				if (typeKind === "primitive")
					serializer = `self.${field} = PrimitiveSerDes::des_rollback(_buffer)?`;
				else serializer = `self.${field}.des_rollback_predict_remove(_buffer)?`; //collections+plugins

				//brackets are a workaround for https://github.com/rust-lang/rust/issues/127436
				if (netVisibility === "private") serializer = `{ ${serializer} }`;

				return `		${netVisibilityAttribute}
		${serializer};`;
			})
			.join("\n\t\t\n"),
	)
	.join("\n\t\t\n")}
		
		Ok(())
	}
}`;

		function canSnapshotNewClient(field: FlattenedField) {
			return (
				field.typeKind !== "struct" &&
				field.netVisibility !== "private" &&
				field.netVisibility !== "untracked"
			);
		}

		function canSnapshotPredictRemove(field: FlattenedField) {
			return (
				!(
					rootStruct.collectionNestDepth === 0 || //skip State. can't delete the entire game
					(rootStruct.collectionNestDepth === 1 && rootStruct.path[1] === "clients") //removal of a client is unrollbackable
				) &&
				field.typeKind !== "struct" &&
				field.netVisibility !== "untracked"
			);
		}
	})
	.join("\n\n")}
`,
	);
}
