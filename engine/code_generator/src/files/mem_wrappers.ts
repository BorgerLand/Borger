import {
	CLIENT_TS_GENERATED_DIR,
	STATE_WARNING,
	getNestedPath,
	isPrimitive,
	type AllFlattenedStructs,
} from "@borger/code_generator/common.ts";
import {
	multiFieldPrimitiveTypeSchema,
	simplePrimitiveTypeSchema,
	type PrimitiveType,
	type SimplePrimitiveType,
} from "@borger/code_generator/state_schema.ts";
import { presentationStructFilter } from "@borger/code_generator/files/presentation.ts";

export function generateMemWrappers(structs: AllFlattenedStructs) {
	const rootInputStruct = structs.input[0];

	Bun.write(
		`${CLIENT_TS_GENERATED_DIR}/mem_wrappers.ts`,
		`${STATE_WARNING}

import * as MemWrappers from "@borger/ts/handwritten/mem_wrappers.ts";
import * as SlotMap from "@borger/ts/networked_types/collections/slotmap.ts";
import * as Primitive from "@borger/ts/networked_types/primitive.ts";
import * as ClientRS from "@borger/rs";

${structs.input
	.map(
		({ name, path }) =>
			`export type ${name} = ReturnType<typeof wrap_Input>${name === "Input" ? `` : `["${getNestedPath(rootInputStruct.path, path).replaceAll(".", '"]["')}"]`}`,
	)
	.join("\n")}

export function wrap_Input(state: MemWrappers.State, ptr: number)
{
	const lifetime = state.curLifetime;
	const offsets = state.offsets.struct.Input;
	
${structs.input
	.reverse()
	.map(
		(struct) =>
			`	const ${struct.name} =
	{
${struct.fields
	.map(function generateInputField({ name, outerType, fullType }) {
		const offset = `offsets.${getNestedPath(rootInputStruct.path, struct.path, name)}`;

		if ((multiFieldPrimitiveTypeSchema.options as string[]).includes(outerType))
			return `		${name}: Primitive.wrap_mut_${outerType}(state, ptr + ${offset}),`;

		if (isPrimitive(outerType)) {
			return `		get_${name}()
		{
			MemWrappers.checkUseAfterFree(state, lifetime);
			return ${getPrimitive(outerType, offset)};
		},
		set_${name}(value: ${(function getSimplePrimitiveType() {
			if (outerType === "bool") return "boolean";
			if (outerType === "char") return "string";
			return "number";
		})()})
		{
			MemWrappers.checkUseAfterFree(state, lifetime);
			${setSimplePrimitive(outerType as SimplePrimitiveType, offset)};
			return this;
		},`;
		}

		return `		${name}: ${fullType},`; //custom struct
	})
	.join("\n\t\t\n")}${
				struct.name === "Input"
					? `
		
		validate()
		{
			ClientRS.validate_input(ptr);
		},`
					: ``
			}
	};`,
	)
	.join("\n\n")}
	
	return Input;
}

export type Output = ReturnType<typeof wrap_Output>;
${structs.sim
	.map(function generateOutputTypes(group) {
		const rootStruct = group[0];
		const rootStructName = getOutputStructName(rootStruct.name);

		return `${group
			.filter(presentationStructFilter)
			.map(function generateOutputTypeDef({ name, path }) {
				const outputStructPath = path.slice();
				if (rootStructName === "Output") outputStructPath.splice(1, 0, "state");
				return `export type ${getOutputStateStructName(name)} = ReturnType<typeof wrap_${rootStructName}>${name === rootStructName ? `` : `["${getNestedPath(rootStruct.path, outputStructPath).replaceAll(".", '"]["')}"]`}`;
			})
			.join("\n")}

${rootStructName === "Output" ? "export " : ""}function wrap_${rootStructName}(state: MemWrappers.State, ptr: number)
{
	const offsets = state.offsets.struct.${rootStructName};
	
${group
	.filter(presentationStructFilter)
	.reverse()
	.map(function generateOutputStruct(struct) {
		const outputStructPath = struct.path.slice();
		if (rootStructName === "Output") outputStructPath.splice(1, 0, "state");

		return `	const ${getOutputStateStructName(struct.name)} =
	{
${struct.fields
	.filter((field) => field.presentation)
	.map(function generateOutputField({ name, outerType, fullType, innerType, isCustomStruct }) {
		if (isCustomStruct) return `		${name}: ${fullType},`;

		const offset = `offsets.${getNestedPath(rootStruct.path, outputStructPath, name)}`;

		if (isPrimitive(outerType)) return `		${name}: ${getPrimitive(outerType, offset)},`;
		if (outerType === "EventDispatcher") return `		${name}: ${getPrimitive("bool", offset)},`;

		if (outerType === "SlotMap")
			return `		${name}: SlotMap.wrap
		(
			state,
			ptr + ${offset},
			state.offsets.slotmap.${innerType},
			wrap_${innerType},
			ClientRS.slotmap_get_${innerType},
		),`;
	})
	.join("\n\t\t\n")}
	};`;
	})
	.join("\n\n")}
	${
		rootStructName === "Output"
			? `
	const Output =
	{
		local_client_id: state.memView.getUint32(ptr + offsets.local_client_id, true),
		
		state: OutputState,
	};
	`
			: ``
	}
	return ${rootStructName};
}`;
	})
	.join("\n\n")}

export enum ClientDiscriminant
{
	Owned = 0,
	Remote = 1,
}

export type Client =
	| { type: ClientDiscriminant.Owned; value: ReturnType<typeof wrap_ClientOwned> }
	| { type: ClientDiscriminant.Remote; value: ReturnType<typeof wrap_ClientRemote> };

function wrap_Client(state: MemWrappers.State, ptr: number): Client
{
	const offsets = state.offsets.struct.Client;
	return state.memView.getUint8(ptr) === ClientDiscriminant.Owned
			? { type: ClientDiscriminant.Owned, value: wrap_ClientOwned(state, ptr + offsets.owned) }
			: { type: ClientDiscriminant.Remote, value: wrap_ClientRemote(state, ptr + offsets.remote) };
}
`,
	);
}

function getPrimitive(type: PrimitiveType, offset: string) {
	if ((simplePrimitiveTypeSchema.options as string[]).includes(type)) {
		const simpleType = type as SimplePrimitiveType;
		let getter: string;

		if (simpleType === "u8") {
			getter = `state.memView.getUint8(ptr + ${offset})`;
		} else if (simpleType === "bool") {
			getter = `state.memView.getUint8(ptr + ${offset}) !== 0`;
		} else if (simpleType === "char") {
			getter = `String.fromCodePoint(state.memView.getUint32(ptr + ${offset}, true))`;
		} else {
			let dataViewMethod: string;
			switch (simpleType) {
				case "i8":
					dataViewMethod = "Int8";
					break;
				case "u16":
					dataViewMethod = "Uint16";
					break;
				case "i16":
					dataViewMethod = "Int16";
					break;
				case "u32":
				case "usize32":
					dataViewMethod = "Uint32";
					break;
				case "i32":
				case "isize32":
					dataViewMethod = "Int32";
					break;
				case "u64":
					dataViewMethod = "Uint64";
					break;
				case "i64":
					dataViewMethod = "Int64";
					break;
				case "f32":
					dataViewMethod = "Float32";
					break;
				case "f64":
					dataViewMethod = "Float64";
					break;
			}

			getter = `state.memView.get${dataViewMethod}(ptr + ${offset}, true)`;
		}

		return getter;
	}

	//multi-field
	return `Primitive.wrap_${type}(state, ptr + ${offset})`;
}

function setSimplePrimitive(type: SimplePrimitiveType, offset: string): string {
	if (type === "u8") {
		return `state.memView.setUint8(ptr + ${offset}, value)`;
	} else if (type === "bool") {
		return `state.memView.setUint8(ptr + ${offset}, value ? 1 : 0)`;
	} else if (type === "char") {
		return `state.memView.setUint32(ptr + ${offset}, value.codePointAt(0)!, true)`;
	} else {
		let dataViewMethod: string;
		switch (type) {
			case "i8":
				dataViewMethod = "Int8";
				break;
			case "u16":
				dataViewMethod = "Uint16";
				break;
			case "i16":
				dataViewMethod = "Int16";
				break;
			case "u32":
			case "usize32":
				dataViewMethod = "Uint32";
				break;
			case "i32":
			case "isize32":
				dataViewMethod = "Int32";
				break;
			case "u64":
				dataViewMethod = "Uint64";
				break;
			case "i64":
				dataViewMethod = "Int64";
				break;
			case "f32":
				dataViewMethod = "Float32";
				break;
			case "f64":
				dataViewMethod = "Float64";
				break;
		}

		return `state.memView.set${dataViewMethod}(ptr + ${offset}, value, true)`;
	}
}

export function getOutputStructName(simStructName: string) {
	return simStructName === "SimulationState" ? "Output" : simStructName;
}

function getOutputStateStructName(simStructName: string) {
	return simStructName === "SimulationState" ? "OutputState" : simStructName;
}
