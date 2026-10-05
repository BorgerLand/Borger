export type Plugin<Name extends string = string> = {
	name: Name;
	tracked: boolean;
	rsSimulationFQN: string; //including the leading "::"
	nodePackageName: string;
	diffOps: string[];

	schemaValidators?: RecursiveSchemaValidator[];

	//remaining fields only needed for presentable types
	rsPresentationOutputFQN?: string; //including the leading "::"
	rsPresentationOutputFieldNames?: [string]; //only needed if there are multiple
	tsMemWrappers?: (offset: string) => string; //see mem_wrappers.ts for example pattern
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
