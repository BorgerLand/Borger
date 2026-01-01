import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	isPrimitive,
	type FlattenedStruct,
	getFullFieldPath,
	type FlattenedField,
} from "@engine/code_generator/Common.ts";

//new client: all public data should be serialized
//predict remove: all locally accessible data should be serialized. "all" has different meanings depending on server/client
export function generateSnapshotSerDesRS(simStructs: FlattenedStruct[][]) {
	Bun.write(
		`${BASE_GENERATED_DIR}/snapshot_serdes.rs`,
		`${STATE_WARNING}

use crate::simulation_state::*;
use crate::DeserializeOopsy;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::snapshot_serdes::SnapshotState;
use crate::networked_types::primitive::usize32;

${simStructs
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
				outerType,
			}) {
				const isClientData = struct.path[1] === "clients";
				const field = getFullFieldPath(rootStruct.path, struct.path, name);

				let serializer;
				if (isPrimitive(outerType)) serializer = `self.${field}.ser_tx(_buffer)`;
				else serializer = `self.${field}.ser_tx_new_client(_client_id, _buffer)`; //collections+utilities

				if (isClientData && netVisibility === "Owner") {
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
			.map(function generateSerializeRemoveField({ name, netVisibilityAttribute, outerType }) {
				const field = getFullFieldPath(rootStruct.path, struct.path, name);

				let serializer;
				if (isPrimitive(outerType)) serializer = `self.${field} = PrimitiveSerDes::des_rx(_buffer)?`;
				else serializer = `self.${field}.des_rx_new_client(_client_id, _buffer)?`; //collections+utilities

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
			.map(function generateSerializeRemoveField({ name, netVisibilityAttribute, outerType }) {
				const field = getFullFieldPath(rootStruct.path, struct.path, name);

				let serializer;
				if (isPrimitive(outerType)) serializer = `self.${field}.ser_rollback(_buffer)`;
				else serializer = `self.${field}.ser_rollback_predict_remove(_buffer)`; //collections+utilities

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
				outerType,
			}) {
				const field = getFullFieldPath(rootStruct.path, struct.path, name);

				let serializer;
				if (isPrimitive(outerType))
					serializer = `self.${field} = PrimitiveSerDes::des_rollback(_buffer)?`;
				else serializer = `self.${field}.des_rollback_predict_remove(_buffer)?`; //collection

				//brackets are a workaround for https://github.com/rust-lang/rust/issues/127436
				if (netVisibility === "Private") serializer = `{ ${serializer} }`;

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
				!field.isCustomStruct &&
				field.netVisibility !== "Private" &&
				field.netVisibility !== "Untracked"
			);
		}

		function canSnapshotPredictRemove(field: FlattenedField) {
			return (
				!(
					rootStruct.collectionNestDepth === 0 || //skip SimulationState. can't delete the entire game
					(rootStruct.collectionNestDepth === 1 && rootStruct.path[1] === "clients") //removal of a client is unrollbackable
				) &&
				!field.isCustomStruct &&
				field.netVisibility !== "Untracked"
			);
		}
	})
	.join("\n\n")}
`,
	);
}
