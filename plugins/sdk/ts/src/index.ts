export type Plugin<Name extends string = string> = {
	name: Name;
	tracked: boolean;
	schemaValidators?: RecursiveSchemaValidator[];
	rustSimFQN: string; //including the leading "::"
	nodePackageName: string;
	diffOps: string[];

	//see mem_offsets.ts for example pattern. only needed if
	//type allows presentation and has multiple fields
	rsMemOffsets?: string;
	//see mem_wrappers.ts for example pattern. only needed if
	//type allows presentation
	tsMemWrappers?: (offset: string) => string;
};

export type RecursiveSchemaValidator = {
	isInvalid: (path: string[], child: Field, parent?: Field) => boolean;
	error: (path: string[], child: Field, parent?: Field) => string;
};

export type Field = {
	netVisibility: NetVisibility;
	type: string;

	presentation?: Presentation;
	typeName?: string;
	content?: string | Struct;
};

export type Struct = Record<string, Field>;

export type NetVisibility =
	| "private" //only server can access
	| "owner" //only server and the owning client can access
	| "public" //everyone can access
	| "untracked"; //disable networking/diff tracking

export type Presentation = "clone" | "interpolate";
