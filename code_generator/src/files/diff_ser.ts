import {
	ENGINE_GENERATED_DIR,
	stateWarningBlock,
	VALID_TYPES,
	getNestedPath,
	nvEnum,
} from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generateDiffSer(flattened: FlattenedOutput) {
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/diff_ser.rs`,
		`${stateWarningBlock()}

use crate::simulation::*;
use crate::diff_ser::{DiffSerializer, ser_sim_primitive};
use borger_plugin_sdk::multiplayer_tradeoff::{AnyTradeOff, DiffSerializerToImpl};

#[cfg(feature = "server")]
use borger_plugin_sdk::NetVisibility;

#[cfg(feature = "client")]
use
{
	crate::diff_ser::ser_input_primitive,
	borger_plugin_sdk::multiplayer_tradeoff::Impl,
};

${VALID_TYPES}

#[cfg(feature = "client")]
pub fn ser_tx_input_diff(old: &Input, new: &Input, diff: &mut DiffSerializer<Impl>)
{
${flattened.input
	.map((struct) =>
		struct.fields
			.filter((field) => field.typeKind === "primitive")
			.map(function generateStructField({ name, fieldID }) {
				const fieldPath = getNestedPath(flattened.input[0].path, struct.path, name);

				return `	if new.${fieldPath} != old.${fieldPath}
	{
		ser_input_primitive(diff, ${fieldID}, new.${fieldPath});
	}`;
			})
			.join("\n\t\n"),
	)
	.join("\n\t\n")}
}

${flattened.output
	.map((group) =>
		group
			.map(function generateSimulationStruct(struct) {
				//primitive fields need setter/getter
				const primitiveFields = struct.fields.filter(
					({ typeKind, netVisibility }) =>
						typeKind === "primitive" && netVisibility !== "untracked",
				);

				return `impl ${struct.name}
{
${primitiveFields
	.map(function generatePrimitiveGetterSetter({
		name,
		netVisibility,
		netVisibilityAttribute,
		outerType,
		fieldID,
	}) {
		const getter =
			`	${netVisibilityAttribute}
` +
			`	pub fn get_${name}(&self) -> ${outerType}
	{
		self.${name}
	}`;

		const setter = `${netVisibilityAttribute}
	pub fn set_${name}(&mut self, value: ${outerType}, diff: &mut DiffSerializer<impl AnyTradeOff>) -> &mut Self
	{
		if value != self.${name}
		{
			ser_sim_primitive
			(
				diff.to_impl(),
				&self._diff_path,
				${fieldID},
				self.${name},
				
				#[cfg(feature = "server")]
				${nvEnum(netVisibility)},
				
				#[cfg(feature = "server")]
				value
			);
			
			self.${name} = value;
		}
		
		self
	}`;

		return `${getter}
	
	${setter}`;
	})
	.join("\n\t\n")}
}`;
			})
			.join("\n\n"),
	)
	.join("\n\n")}
`,
	);
}
