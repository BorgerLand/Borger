import type { Field, NetVisibility, Presentation, Struct, Plugin } from "@borger/plugin_sdk";
import { nvEnum } from "@borger/code_generator/common.ts";
import {
	collectionTypeSchema,
	primitiveTypeSchema,
	type PrimitiveType,
} from "@borger/code_generator/state_schema.ts";

export type FlattenedOutput = {
	output: FlattenedStruct[][]; //inner layer = structs that are grouped in the same diff path, outer layer = all
	input: FlattenedStruct[];
	plugins: Plugin[];
};

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
	netVisibility: NetVisibility;
	netVisibilityAttribute: string;
	presentation?: Presentation;
	fieldID: number | "N/A";
} & (
	| {
			typeKind: "struct" | "external";
			outerType: string; //if external, this is an fqn, if struct then nope
			innerType?: never;
			plugin?: never;
	  }
	| {
			typeKind: "primitive";
			outerType: PrimitiveType;
			innerType?: never;
			plugin?: never;
	  }
	| {
			typeKind: "plugin";
			outerType: string; //fqn of the type
			innerType?: never;
			plugin: Plugin;
	  }
	| {
			typeKind: "collection";
			outerType: string; //fqn of the type, excluding generic params
			innerType: string; //inner generic param type inside the <>
			plugin?: never;
	  }
);

type ClientKind = "NA" | "Owned" | "Remote";

//recursively traverse the state object and "flatten" it into a big list of structs
export function flatten(
	parentStruct: Struct,
	plugins: Plugin[],
	parentPath: string[] = ["state"],
	parentField?: Field,
	parentClientKind: ClientKind = "NA",
	structsFlattened: FlattenedOutput = {
		output: [[]],
		input: [],
		plugins,
	},
	diffPathInfo: { path: (string | number)[]; depth: number; structGroupID: number; fieldID: number } = {
		path: [],
		depth: 0,
		structGroupID: 0,
		fieldID: 0,
	},
) {
	const parentStructFlattened: FlattenedStruct = {
		name: generateStructName(parentField?.typeName ?? pathToStructName(parentPath), parentClientKind),
		path: parentPath,
		netVisibility: parentField?.netVisibility ?? "public",
		clientKind: parentClientKind,
		fields: [],
		collectionNestDepth: diffPathInfo.depth,
	};

	if (parentPath[0] === "state")
		structsFlattened.output[diffPathInfo.structGroupID].push(parentStructFlattened);
	else structsFlattened.input.push(parentStructFlattened);

	for (const [childFieldName, childField] of Object.entries(parentStruct)) {
		const netVisibility = childField.netVisibility;
		let childClientKind = parentClientKind;

		let netVisibilityAttribute;

		//even if the field is skipped due to having no
		//net visibility, still need to traverse in
		//order to populate fieldID accurately
		let skipGeneratingField = false;
		const fieldID =
			childField.type === "struct" || childField.netVisibility === "untracked"
				? "N/A"
				: diffPathInfo.fieldID++;
		const diffPath = [...diffPathInfo.path, fieldID];
		const formattedDiffPath = fieldID === "N/A" ? "" : `, diff path [${diffPath.join(", ")}]`;

		if (childField.netVisibility === "untracked") {
			netVisibilityAttribute = "//Untracked";
		} else if (childClientKind === "NA" || childClientKind === "Owned") {
			const comment = `//ClientKind::${childClientKind}, ${nvEnum(netVisibility)}${formattedDiffPath}`;

			//global or local client owned
			if (netVisibility === "public") netVisibilityAttribute = comment;
			else if (netVisibility === "owner") netVisibilityAttribute = comment;
			else if (netVisibility === "private")
				netVisibilityAttribute = `#[cfg(feature = "server")] ${comment}`;
		} else {
			//local client remote
			if (netVisibility === "public")
				netVisibilityAttribute = `//ClientKind::Remote, NetVisibility::Public${formattedDiffPath}`;
			else if (netVisibility === "owner") skipGeneratingField = true;
			else if (netVisibility === "private") skipGeneratingField = true;
		}

		let childFieldFlattened: FlattenedField | undefined;
		if (!skipGeneratingField) {
			const childFieldFlattenedCommon = {
				name: childFieldName,
				presentation: childField.presentation,
				netVisibility: childField.netVisibility,
				netVisibilityAttribute: netVisibilityAttribute!,
				fieldID,
			} as const;

			const plugin = plugins.find((plugin) => plugin.name === childField.type);
			if (plugin) {
				childFieldFlattened = {
					...childFieldFlattenedCommon,
					typeKind: "plugin",
					outerType: plugin.rustSimFQN,
					plugin,
				};
			} else {
				const primitiveParse = primitiveTypeSchema.safeParse(childField.type);
				if (primitiveParse.success) {
					childFieldFlattened = {
						...childFieldFlattenedCommon,
						typeKind: "primitive",
						outerType: primitiveParse.data,
					};
				} else {
					childFieldFlattened = {
						...childFieldFlattenedCommon,
						typeKind: "external", // compute struct+collection later and overwrite
						outerType: childField.type,
					};
				}
			}

			parentStructFlattened.fields.push(childFieldFlattened);
		}

		if (collectionTypeSchema.safeParse(childField.type).success || childField.type === "struct") {
			//field has nested data (child struct or collection)
			let childPath = [...parentPath, childFieldName];
			const childBaseTypeName = childField.typeName ?? pathToStructName(childPath);
			const isOwnableStruct = childBaseTypeName === "Input";
			let skipRemoteVariant = false;

			if (isOwnableStruct) {
				//disable owner/remote suffix. the struct is agnostic to scope
				if (childClientKind === "Remote") skipRemoteVariant = true;
				childClientKind = "NA";
			}

			if (childFieldFlattened) {
				if (childField.type === "struct") {
					childFieldFlattened.typeKind = "struct";
					childFieldFlattened.outerType = generateStructName(childBaseTypeName, childClientKind);
				} else {
					//the content must always be a struct, even if the content
					//was declared as a string. this gives the _diff_path field
					//and state-tracking setter method a home
					childFieldFlattened.typeKind = "collection";
					childFieldFlattened.outerType = childField.type;
					childFieldFlattened.innerType = generateStructName(childBaseTypeName, childClientKind);
				}
			}

			if (skipRemoteVariant) continue; //avoid generating remote variant. only need 1 struct
			if (childBaseTypeName === "Input") childPath = ["input"]; //swap from generating (output) state to generating input state

			let childStruct: Struct;
			if (typeof childField.content === "object") {
				childStruct = childField.content;
			} else {
				//wrap primitive/plugin field in a single-field struct.
				//collection's value must implement TrackedState trait,
				//which can only be implemented by a struct
				childStruct = {
					value: {
						netVisibility: childField.netVisibility,
						presentation: childField.presentation,
						type: childField.content!,
					},
				};
			}

			function getChildDiffPathInfo() {
				if (childBaseTypeName === "Input") {
					return { path: [], depth: 0, structGroupID: 0, fieldID: 0 }; //start over from scratch
				} else if (childField.type === "struct") {
					return diffPathInfo; //different struct but still within the same block of contiguous memory
				} else {
					if (diffPathInfo.depth === 256) {
						//diff ser stores path depth as u8
						throw Error("Too many stinkin' nested collections (max 256)");
					}

					//restart field id counter from 0 upon encountering
					//a collection, which will start a new struct group
					//and append 2 more elements to the _diff_path
					//(field id, element id)
					return {
						path: [...diffPath, "x"],
						depth: diffPathInfo.depth + 1,
						structGroupID: structsFlattened.output.push([]) - 1,
						fieldID: 0,
					};
				}
			}

			if (childBaseTypeName === "Client") {
				//branch off twice to generate separate
				//owned+remote client structs
				flatten(
					childStruct,
					plugins,
					childPath,
					childField,
					"Owned",
					structsFlattened,
					getChildDiffPathInfo(),
				);
				flatten(
					childStruct,
					plugins,
					childPath,
					childField,
					"Remote",
					structsFlattened,
					getChildDiffPathInfo(),
				);
			} else {
				flatten(
					childStruct,
					plugins,
					childPath,
					childField,
					childClientKind,
					structsFlattened,
					getChildDiffPathInfo(),
				);
			}
		}
	}

	return structsFlattened;
}

function generateStructName(baseTypeName: string, clientKind: ClientKind) {
	switch (clientKind) {
		case "Owned":
			return `${baseTypeName}Owned`;
		case "Remote":
			return `${baseTypeName}Remote`;
		default:
			return baseTypeName;
	}
}

//converts ["my_string", "_another__string", "lot_of_strings"]
//to MyString_AnotherString_LotOfStrings
function pathToStructName(path: string[]) {
	return path
		.map((str) =>
			str
				.split("_")
				.filter((word) => word.length > 0)
				.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
				.join(""),
		)
		.join("_");
}
