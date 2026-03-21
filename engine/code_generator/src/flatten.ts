import { type Field, type Struct } from "@borger/code_generator/state_schema.ts";
import type {
	FlattenedStruct,
	AllFlattenedStructs,
	FlattenedField,
	DiffPath,
	ClientKind,
} from "@borger/code_generator/common.ts";
import { isGeneric, nvEnum } from "@borger/code_generator/common.ts";

//recursively traverse the state object and "flatten" it
//into a big list of structs
export function flatten(
	parentStruct: Struct,
	parentPath: string[] = ["simulation_state"], //pathToStructName will change this to SimulationState
	parentField?: Field,
	parentClientKind: ClientKind = "NA",
	structsFlattened: AllFlattenedStructs = {
		sim: [[]],
		input: [],
	},
	diffPathInfo: { path: DiffPath; depth: number; structGroupID: number; fieldID: number } = {
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

	if (parentPath[0] === "simulation_state")
		structsFlattened.sim[diffPathInfo.structGroupID].push(parentStructFlattened);
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
			//default to treating this field as primitive data.
			//go back and change it later if needed
			childFieldFlattened = {
				name: childFieldName,
				outerType: childField.type,
				fullType: childField.type,
				innerType: childField.type,
				isCustomStruct: childField.type === "struct",
				presentation: childField.presentation,
				netVisibility: childField.netVisibility,
				netVisibilityAttribute: netVisibilityAttribute!,
				fieldID,
			};

			parentStructFlattened.fields.push(childFieldFlattened);
		}

		if (
			(isGeneric(childField.type) || childField.type === "struct") &&
			childField.netVisibility !== "untracked"
		) {
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

			if (!skipGeneratingField && /*redundant:*/ childFieldFlattened) {
				if (childField.type === "struct") {
					//static struct
					childFieldFlattened.fullType =
						childFieldFlattened.outerType =
						childFieldFlattened.innerType =
							generateStructName(childBaseTypeName, childClientKind);
				} else {
					//collection/dynamic allocation - the content must
					//always be a struct, even if the declaration only
					//requests a primitive. this gives the _diff_path
					//field and state-tracking setter method a home
					const innerType = generateStructName(childBaseTypeName, childClientKind);
					childFieldFlattened.fullType = `${childField.type}<${innerType}>`;
					childFieldFlattened.outerType = childField.type;
					childFieldFlattened.innerType = innerType;
				}
			}

			if (skipRemoteVariant) continue; //avoid generating remote variant. only need 1 struct
			if (childBaseTypeName === "Input") childPath = ["input_state"]; //swap from generating simulation state to generating input state

			let childStruct: Struct;
			if (typeof childField.content === "object") {
				childStruct = childField.content;
			} else {
				//wrap primitive/utility field in a single-field struct.
				//collection's value must implement TrackedState trait,
				//which can only be implemented by a struct
				childStruct = {
					value: {
						netVisibility: childField.netVisibility,
						presentation: childField.presentation,
						type: childField.content!,
					} as Field,
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
						structGroupID: structsFlattened.sim.push([]) - 1,
						fieldID: 0,
					};
				}
			}

			if (childBaseTypeName === "Client") {
				//branch off twice to generate separate
				//owned+remote client structs
				flatten(
					childStruct,
					childPath,
					childField,
					"Owned",
					structsFlattened,
					getChildDiffPathInfo(),
				);
				flatten(
					childStruct,
					childPath,
					childField,
					"Remote",
					structsFlattened,
					getChildDiffPathInfo(),
				);
			} else {
				flatten(
					childStruct,
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
		.join("_"); // Join the processed strings with underscores
}
