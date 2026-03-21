import { z } from "zod";
import { isGeneric } from "@borger/code_generator/common.ts";

//true single-field primitives that can be easily read
//directly from wasm memory
export const simplePrimitiveTypeSchema = z.enum([
	"bool",
	"u8",
	"i8",
	"u16",
	"i16",
	"u32",
	"i32",
	"u64",
	"i64",
	"f32",
	"f64",
	"char", //utf-32
	"usize32",
	"isize32",
]);

export const multiFieldPrimitiveTypeSchema = z.enum([
	"Vec2", //xy, f32
	"DVec2", //xy, f64
	"Vec3", //xyz, f32
	"DVec3", //xyz, f64
	"Quat", //xyzw, f32
	"DQuat", //xyzw, f64
]);

//the subset of primitives consisting of floats, containing
//both simple+multi-field
export const interpolablePrimitiveTypeSchema = z.enum([
	"f32",
	"f64",
	"Vec2",
	"DVec2",
	"Vec3",
	"DVec3",
	"Quat",
	"DQuat",
]) satisfies z.ZodType<PrimitiveType>;

export const primitiveTypeSchema = z.enum([
	...simplePrimitiveTypeSchema.options,
	...multiFieldPrimitiveTypeSchema.options,
]);
export const collectionTypeSchema = z.enum(["SlotMap"]);
export const utilityTypeSchema = z.enum(["EventDispatcher"]);

//all types
export const typeSchema = z.enum([
	...primitiveTypeSchema.options,
	...collectionTypeSchema.options,
	...utilityTypeSchema.options,
]);

//"generic type" for now just means a type with one generic param
export const genericTypeSchema = z.enum([...collectionTypeSchema.options]);
//unfortunately zod does not seem to have an enum subtraction method
//equivalent to typescript Exclude<>
export const nonGenericTypeSchema = z.enum(
	typeSchema.options.filter((type) => !genericTypeSchema.options.includes(type as any)),
) as z.ZodEnum<{ [K in NonGenericType]: K }>;

export const netVisibilitySchema = z.enum([
	"private", //only server can access
	"owner", //only server and the owning client can access
	"public", //everyone can access
	"untracked", //disable networking/diff tracking
]); //in order from least to most to make comparisons easier

export const presentationSchema = z.enum(["clone", "interpolate"]);

export type NetVisibility = z.infer<typeof netVisibilitySchema>;
export type Presentation = z.infer<typeof presentationSchema>;

//- type
//	- generic
//		- collection
//	- non-generic
//		- primitive
//			- simple primitive (some are interpolable)
//			- multi-field primitive (some are interpolable)
//  - utilities may or may not have generic params
export type Type = z.infer<typeof typeSchema>;
export type GenericType = z.infer<typeof genericTypeSchema>;
export type NonGenericType = Exclude<Type, GenericType>;
export type PrimitiveType = z.infer<typeof primitiveTypeSchema>;
export type InterpolablePrimitiveType = z.infer<typeof interpolablePrimitiveTypeSchema>;
export type SimplePrimitiveType = z.infer<typeof simplePrimitiveTypeSchema>;
export type MultiFieldPrimitiveType = z.infer<typeof multiFieldPrimitiveTypeSchema>;
export type CollectionType = z.infer<typeof collectionTypeSchema>;
export type UtilityType = z.infer<typeof utilityTypeSchema>;

const rustIdentifier = z.string().regex(/^[a-zA-Z_][a-zA-Z0-9_]*$/);

const fieldSchema = z.lazy(() =>
	z.union([
		//NETWORKED
		z
			.union([
				z.object({
					netVisibility: z.enum(
						netVisibilitySchema.options.filter((val) => !(val === "untracked")),
					),
					presentation: z.never().optional(),
				}),
				z.object({
					//disallow presentation on private state
					netVisibility: z.enum(
						netVisibilitySchema.options.filter(
							(val) => !(val === "private" || val === "untracked"),
						),
					),
					presentation: presentationSchema,
				}),
			])
			.and(
				z.union([
					z.object({
						type: interpolablePrimitiveTypeSchema,
						typeName: z.never().optional(),
						content: z.never().optional(),
						presentation: presentationSchema,
					}),
					z.object({
						type: nonGenericTypeSchema,
						typeName: z.never().optional(),
						content: z.never().optional(),
						presentation: z.literal("clone").optional(),
					}),
					z.object({
						type: z.literal("struct"),
						typeName: rustIdentifier.optional(),
						content: structSchema,
						presentation: z.literal("clone").optional(),
					}),
					z.object({
						type: genericTypeSchema,
						typeName: rustIdentifier.optional(),
						content: z.union([nonGenericTypeSchema, structSchema]),
						presentation: z.literal("clone").optional(),
					}),
				]),
			),
		//UNTRACKED
		z
			.object({
				netVisibility: z.literal("untracked"),
				typeName: z.never().optional(),
				content: z.never().optional(),
			})
			.and(
				z.union([
					z.object({
						type: interpolablePrimitiveTypeSchema,
						presentation: presentationSchema,
					}),
					z.object({
						type: typeSchema,
						presentation: z.literal("clone"),
					}),
					z.object({
						type: z.string(),
						presentation: z.never().optional(),
					}),
				]),
			),
	]),
) as z.ZodType<Field>;

//need to manually specify field's type because
//z.infer doesn't work on recursive structures
export type Field =
	//NETWORKED
	| ((
			| {
					netVisibility: Exclude<NetVisibility, "untracked">;
					presentation?: never;
			  }
			| {
					//disallow presentation on private state
					netVisibility: Exclude<NetVisibility, "private" | "untracked">;
					presentation: Presentation;
			  }
	  ) &
			(
				| {
						type: InterpolablePrimitiveType;
						typeName?: never;
						content?: never;
						presentation: Presentation;
				  }
				| {
						type: NonGenericType;
						typeName?: never;
						content?: never;
						presentation?: "clone";
				  }
				| {
						type: "struct";
						typeName?: string;
						content: Struct;
						presentation?: "clone";
				  }
				| {
						type: GenericType;
						typeName?: string;
						content: NonGenericType | Struct;
						presentation?: "clone";
				  }
			))
	//UNTRACKED
	| ({
			netVisibility: "untracked";
			typeName?: never;
			content?: never;
	  } & (
			| {
					type: InterpolablePrimitiveType;
					presentation: Presentation;
			  }
			| {
					type: Type;
					presentation: "clone";
			  }
			| {
					//- must specify fully qualified name if not one of the
					//code generator-recognized primitive/utility/collection
					//types. note chosen type currently can't contain generic
					//params <>
					//- chosen type must either be Debug+Default OR Debug+
					//UntrackedState+contain a `pub(crate) fn default() -> Self`
					//method not associated with the Default trait.
					type: string;
					presentation?: never;
			  }
	  ));

const structSchema = z.lazy(() => z.record(rustIdentifier, fieldSchema));
export type Struct = z.infer<typeof structSchema>;

const clientsSlotMapSchema = fieldSchema.and(
	z.object({
		netVisibility: z.literal("public"),
		type: z.literal("SlotMap"),
		typeName: z.literal("Client"),
		content: structSchema.and(
			z.object({
				input: fieldSchema.and(
					z.object({
						netVisibility: z.literal("owner"),
						presentation: z.never().optional(),
						type: z.literal("struct"),
						typeName: z.literal("Input"),
					}),
				),
			}),
		),
	}),
);

function validateRecursively({
	struct,
	test,
	error,
}: {
	struct: Struct;
	test: (path: string[], child: Field, parent?: Field) => boolean; //true on fail
	error: (path: string[], child: Field, parent?: Field) => string;
}) {
	return traverse(struct);
	function traverse(childStruct: Struct, parentField?: Field, parentPath: string[] = []): boolean {
		for (const [childFieldName, childField] of Object.entries(childStruct)) {
			const childPath = [...parentPath, childFieldName];

			if (test(childPath, childField, parentField)) {
				//eslint-disable-next-line no-console
				console.error(error(childPath, childField, parentField));
				return false;
			}

			if (
				childField.type === "struct" ||
				(isGeneric(childField.type) && typeof childField.content === "object")
			) {
				if (!traverse(childField.content as Struct, childField, childPath)) {
					return false;
				}
			}
		}

		return true;
	}
}

const simulationStateSchema = structSchema
	//the simulationStateSchema is the top level structSchema
	//with a mandatory "clients" field
	.and(
		z.object({
			clients: clientsSlotMapSchema, //must contain client state
		}),
	)
	//extra constraints not enforceable through zod's standard api/typescript
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) => path[0] !== "clients" && child.netVisibility === "owner",
			error: (path) => `"owner" visibility used outside of clients for "${path.join(".")}"`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: function (path, child, parent) {
				if (!parent) return false;

				const hierarchy = netVisibilitySchema.options;
				return (
					hierarchy.indexOf(child.netVisibility) > hierarchy.indexOf(parent.netVisibility) &&
					child.netVisibility !== "untracked"
				);
			},
			error: (path, child, parent) =>
				`Net visibility "${child.netVisibility}" for "${path.join(".")}" is more permissive than parent "${parent!.netVisibility}"`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) =>
				path[0] === "clients" && path[1] === "input" && child.netVisibility !== "owner",
			error: (path, child) =>
				`Client input state's net visibility "${child.netVisibility}" for "${path.join(".")}" must be changed to "owner"`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) =>
				path[0] === "clients" &&
				path[1] === "input" &&
				((collectionTypeSchema.options as string[]).includes(child.type) ||
					(utilityTypeSchema.options as string[]).includes(child.type)),
			error: (path, child) =>
				`Client input state's type "${child.type}" for "${path.join(".")}" can't be a utility or collection type`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) =>
				path[0] === "clients" && path[1] === "input" && Boolean(child.presentation),
			error: (path) => `Client input state "${path.join(".")}" can't use presentation: true`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child, parent) => Boolean(child.presentation && parent && !parent.presentation),
			error: (path) =>
				`In order to enable presentation on "${path.join(".")}", its parent must also have presentation enabled`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) => child.type === "EventDispatcher" && !child.presentation,
			//this is not a hard technical requirement, but it makes no sense
			//to use event dispathcer in this manner
			error: (path) => `EventDispatcher at "${path.join(".")}" must have presentation enabled`,
		}),
	);

export type SimulationState = z.infer<typeof simulationStateSchema>;

export function validate(state: unknown) {
	return simulationStateSchema.parse(state);
}
