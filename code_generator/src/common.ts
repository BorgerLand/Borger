import type { NetVisibility } from "@borger/plugin_sdk";
import type { FlattenedStruct } from "@borger/code_generator/flatten.ts";

export const ENGINE_DIR = "borger/engine";
export const ENGINE_GENERATED_DIR = "borger/engine/src/generated";
export const CLIENT_RS_GENERATED_DIR = "borger/client/rs/src/generated";
export const CLIENT_TS_GENERATED_DIR = "borger/client/ts/src/generated";

const STATE_WARNING = `This file was flatulated out by the code generator.
It is auto-generated, so any changes to the file will be overwritten.
Edit /src/state.ts instead!`;

export function stateWarningBlock() {
	return `/*
${STATE_WARNING}
*/`;
}

export function stateWarningHash() {
	return STATE_WARNING.split("\n")
		.map((line) => `#${line}`)
		.join("\n");
}

export const VALID_TYPES = `use
{
	glam::{Vec2, DVec2, Vec3, DVec3, Quat, DQuat},
	borger_plugin_sdk::primitive::{usize32, isize32},
	crate::plugins::slotmap::SlotMap,
};`;

export function rsFQNtoCrateName(rsFQN: string) {
	return rsFQN.split("::")[1];
}

//this is specifically for typeKind: "external", which will
//still have generic params inside of its outerType
//before: ::example::TypeName<GenericParam>
//after:  ::example::TypeName
export function removeGenerics(type: string) {
	return type.split("<", 1)[0].trim();
}

export function presentationStructFilter(struct: FlattenedStruct) {
	return !(
		(struct.clientKind === "Remote" && struct.netVisibility !== "public") ||
		struct.netVisibility === "private"
	);
}

/*
baseGroupPath: ["state", "x", "y"]
fullPath: ["state", "x", "y"]
returns: fieldName

baseGroupPath: ["state"]
fullPath: ["state", "x", "y"]
returns: x.y.fieldName
*/
export function getNestedPath(baseGroupPath: string[], fullPath: string[], fieldName?: string) {
	const segments = fullPath.slice(baseGroupPath.length);
	return [...segments, ...(fieldName ? [fieldName] : [])].join(".");
}

export function nvEnum(variant: NetVisibility) {
	return `NetVisibility::${variant[0].toUpperCase() + variant.slice(1)}`;
}
