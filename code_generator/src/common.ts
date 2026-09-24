import type { NetVisibility, Plugin } from "@borger/plugin_sdk";

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

export function pluginCrateName(plugin: Plugin) {
	return plugin.rustSimFQN.split("::").find((segment) => segment.length > 0);
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
