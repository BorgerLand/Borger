import {
	BASE_GENERATED_DIR,
	STATE_WARNING,
	VALID_TYPES,
	isPrimitive,
	type AllFlattenedStructs,
	getFullFieldPath,
} from "@engine/code_generator/common.ts";

export function generateDiffSerRS(structs: AllFlattenedStructs) {
	Bun.write(
		`${BASE_GENERATED_DIR}/diff_ser.rs`,
		`${STATE_WARNING}

use crate::simulation_state::*;
use crate::diff_ser::DiffSerializer;
use crate::networked_types::primitive::ser_sim_primitive;
use crate::context::AnyTradeoff;

#[cfg(feature = "server")]
use crate::NetVisibility;

#[cfg(feature = "client")]
use
{
	crate::networked_types::primitive::ser_input_primitive,
	crate::context::Impl,
};

${VALID_TYPES}

#[cfg(feature = "client")]
pub fn ser_tx_input_diff(old: &InputState, new: &InputState, diff: &mut DiffSerializer<Impl>)
{
${structs.input
	.map((struct) =>
		struct.fields
			.filter((field) => isPrimitive(field.outerType))
			.map(function generateStructField({ name, fieldID }) {
				const fieldPath = getFullFieldPath(structs.input[0].path, struct.path, name);

				return `	if new.${fieldPath} != old.${fieldPath}
	{
		ser_input_primitive(diff, ${fieldID}, new.${fieldPath});
	}`;
			})
			.join("\n\t\n"),
	)
	.join("\n\t\n")}
}

${structs.sim
	.map((group) =>
		group
			.map(function generateSimulationStruct(struct) {
				//primitive fields need setter/getter
				const primitiveFields = struct.fields.filter(
					(field) => isPrimitive(field.outerType) && field.netVisibility !== "Untracked",
				);

				return `impl ${struct.name}
{
${primitiveFields
	.map(function generatePrimitiveGetterSetter({
		name,
		netVisibility,
		netVisibilityAttribute,
		fullType,
		fieldID,
	}) {
		const getter =
			`	${netVisibilityAttribute}
` +
			`	pub fn get_${name}(&self) -> ${fullType}
	{
		self.${name}
	}`;

		const setter = `${netVisibilityAttribute}
	pub fn set_${name}(&mut self, value: ${fullType}, diff: &mut DiffSerializer<impl AnyTradeoff>) -> &mut Self
	{
		if value != self.${name}
		{
			#[cfg(feature = "server")]
			ser_sim_primitive(diff.to_impl(), &self._diff_path, ${fieldID}, self.${name}, NetVisibility::${netVisibility}, value);
			#[cfg(feature = "client")]
			ser_sim_primitive(diff.to_impl(), &self._diff_path, ${fieldID}, self.${name});
			
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
