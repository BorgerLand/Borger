import {
	collectionTypeSchema,
	genericTypeSchema,
	primitiveTypeSchema,
	utilityTypeSchema,
	type CollectionType,
	type GenericType,
	type NetVisibility,
	type Presentation,
	type PrimitiveType,
	type UtilityType,
} from "@borger/code_generator/state_schema.ts";

export const BORGER_GENERATED_DIR = "../borger/src/generated";
export const CLIENT_RS_GENERATED_DIR = "../client/rs/src/generated";
export const CLIENT_TS_GENERATED_DIR = "../client/ts/src/generated";

export const STATE_WARNING = `/*
This file was flatulated out by the code generator.
It is auto-generated, so any changes to the file will be overwritten.
Edit /game/State.ts instead!
*/`;

export const VALID_TYPES = `#[allow(unused_imports)]
use
{
	glam::{Vec2, DVec2, Vec3, DVec3, Quat, DQuat},
	crate::networked_types::primitive::{usize32, isize32},
	crate::networked_types::collections::slotmap::SlotMap,
	crate::networked_types::event_dispatcher::EventDispatcher,
};`;

export type ClientKind = "NA" | "Owned" | "Remote";

export type FlattenedStruct = {
	name: string;
	path: string[];
	clientKind: ClientKind;
	netVisibility: NetVisibility;
	fields: FlattenedField[];
	collectionNestDepth: number;
};

export type FlattenedField = {
	name: string;

	//fullType = outerType<innerType>

	//eg. u8, f32, MyStruct, SlotMap<f32> (contains generic
	//params)
	fullType: string;

	//eg. u8, f32, MyStruct, SlotMap, etc. (fullType but without
	//the generic param. need this because concatting fullType
	//like SlotMap<u8>::default() is a syntax error, and makes
	//it harder for the generator to check if a type is a
	//collection)
	outerType: string;

	//eg. u8, f32, MyStruct, MySlotMapElement (either the
	//custom name taken straight from the declaration, an
	//auto generated one based on path, or a primitive or
	//utility type if not a struct)
	innerType: string;

	netVisibility: NetVisibility;
	netVisibilityAttribute: string;
	isCustomStruct: boolean;
	presentation?: Presentation;
	fieldID: number | "N/A";
};

export type DiffPath = (string | number)[];

export type AllFlattenedStructs = {
	sim: FlattenedStruct[][]; //inner layer = structs that are grouped in the same diff path, outer layer = all
	input: FlattenedStruct[];
};

//should be able to pass in fullType/innerType too.
//just chose outerType to match the other isType api
export function isPrimitive(outerType: string): outerType is PrimitiveType {
	return (primitiveTypeSchema.options as string[]).includes(outerType);
}

export function isCollection(outerType: string): outerType is CollectionType {
	return (collectionTypeSchema.options as string[]).includes(outerType);
}

export function isUtility(outerType: string): outerType is UtilityType {
	return (utilityTypeSchema.options as string[]).includes(outerType);
}

export function isGeneric(outerType: string): outerType is GenericType {
	return (genericTypeSchema.options as string[]).includes(outerType);
}

/*
baseGroupPath: ["simulation_state", "x", "y"]
fullPath: ["simulation_state", "x", "y"]
returns: fieldName

baseGroupPath: ["simulation_state"]
fullPath: ["simulation_state", "x", "y"]
returns: x.y.fieldName
*/
export function getNestedPath(baseGroupPath: string[], fullPath: string[], fieldName?: string) {
	const segments = fullPath.slice(baseGroupPath.length);
	return [...segments, ...(fieldName ? [fieldName] : [])].join(".");
}

export function nvEnum(variant: NetVisibility) {
	return `NetVisibility::${variant[0].toUpperCase() + variant.slice(1)}`;
}
