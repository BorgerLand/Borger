import { z } from "zod";
import { isGeneric } from "@engine/code_generator/common.ts";

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

const multiFieldPrimitiveTypeSchema = z.enum([
	"Vec2", //xy, f32
	"DVec2", //xy, f64
	"Vec3", //xyz, f32
	"DVec3", //xyz, f64
	"Quat", //xyzw, f32
	"DQuat", //xyzw, f64
]);

export const primitiveTypeSchema = z.enum([
	...simplePrimitiveTypeSchema.options,
	...multiFieldPrimitiveTypeSchema.options,
]);
export const collectionTypeSchema = z.enum(["SlotMap"]);
export const utilityTypeSchema = z.enum(["HapticPredictionEmitter"]);
const typeSchema = z.enum([
	...primitiveTypeSchema.options,
	...collectionTypeSchema.options,
	...utilityTypeSchema.options,
]);

//"generic type" for now just means a type with one generic param
export const genericTypeSchema = z.enum([...collectionTypeSchema.options, "HapticPredictionEmitter"]);
//unfortunately zod does not seem to have an enum subtraction method
export const nonGenericTypeSchema = z.enum(
	typeSchema.options.filter((type) => !genericTypeSchema.options.includes(type as any)),
) as z.ZodEnum<{ [K in NonGenericType]: K }>;

export const netVisibilitySchema = z.enum([
	"Private", //only server can access
	"Owner", //only server and the owning client can access
	"Public", //everyone can access
	"Untracked", //disable networking/diff tracking
]); //in order from least to most to make comparisons easier

export type NetVisibility = z.infer<typeof netVisibilitySchema>;

//- type
//	- primitive
//		- simple primitive
//		- multi field primitive
//	- collection
//	- utility
export type Type = z.infer<typeof typeSchema>;
export type PrimitiveType = z.infer<typeof primitiveTypeSchema>;
export type SimplePrimitiveType = z.infer<typeof simplePrimitiveTypeSchema>;
export type MultiFieldPrimitiveType = z.infer<typeof multiFieldPrimitiveTypeSchema>;
export type CollectionType = z.infer<typeof collectionTypeSchema>;
export type UtilityType = z.infer<typeof utilityTypeSchema>;

export type GenericType = z.infer<typeof genericTypeSchema>;
export type NonGenericType = Exclude<Type, GenericType>;

const fieldSchema = z.lazy(() =>
	z.union([
		z
			.union([
				z.object({
					netVisibility: z.enum(
						netVisibilitySchema.options.filter((val) => !(val === "Untracked")),
					),
					presentation: z.literal(false).optional(),
				}),
				z.object({
					netVisibility: z.enum(
						netVisibilitySchema.options.filter(
							(val) => !(val === "Private" || val === "Untracked"),
						),
					),
					presentation: z.literal(true),
				}),
			])
			.and(
				z.union([
					z.object({
						type: nonGenericTypeSchema,
						typeName: z.never().optional(),
						content: z.never().optional(),
					}),
					z.object({
						type: z.literal("struct"),
						typeName: z.string().optional(),
						content: structSchema,
					}),
					z.object({
						type: genericTypeSchema,
						typeName: z.string().optional(),
						content: z.union([nonGenericTypeSchema, structSchema]),
					}),
				]),
			),
		z
			.object({
				netVisibility: z.literal("Untracked"),
				typeName: z.never().optional(),
				content: z.never().optional(),
			})
			.and(
				z.union([
					z.object({
						presentation: z.literal(false).optional(),
						type: z.string(),
					}),
					z.object({
						presentation: z.literal(true),
						type: typeSchema,
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
					netVisibility: Exclude<NetVisibility, "Untracked">;
					presentation?: false;
			  }
			| {
					//disallow { netVisibility: "Private", presentation: true }
					netVisibility: Exclude<NetVisibility, "Private" | "Untracked">;
					presentation: true;
			  }
	  ) &
			(
				| {
						type: NonGenericType;
						typeName?: never;
						content?: never;
				  }
				| {
						type: "struct";
						typeName?: string;
						content: Struct;
				  }
				| {
						type: GenericType;
						typeName?: string;
						content: NonGenericType | Struct;
				  }
			))
	//UNTRACKED
	| ({
			netVisibility: "Untracked";
			typeName?: never;
			content?: never;
	  } & (
			| {
					presentation?: false;

					//- must specify fully qualified name if not one of the
					//code generator-recognized primitive/utility/collection
					//types. note chosen type currently can't contain generic
					//params <>
					//- for use in haptic prediction: chosen type must implement
					//Debug+Serialize+Deserialize. Clone not required
					//- for other uses: chosen type must either be Debug+Default
					//OR Debug+UntrackedState+contain a
					//`pub(crate) fn default() -> Self` method not associated
					//with the Default trait.
					type: string;
			  }
			| {
					presentation: true;
					type: Type;
			  }
	  ));

const structSchema = z.lazy(() => z.record(z.string().regex(/^[a-zA-Z_][a-zA-Z0-9_]*$/), fieldSchema));
export type Struct = z.infer<typeof structSchema>;

const clientsSlotMapSchema = fieldSchema.and(
	z.object({
		netVisibility: z.literal("Public"),
		type: z.literal("SlotMap"),
		typeName: z.literal("ClientState"),
		content: structSchema.and(
			z.object({
				input: fieldSchema.and(
					z.object({
						netVisibility: z.literal("Owner"),
						presentation: z.literal(false).optional(),
						type: z.literal("struct"),
						typeName: z.literal("InputState"),
					}),
				),
			}),
		),
	}),
);

const entitySlotMapSchema = fieldSchema.and(
	z.object({
		netVisibility: z.literal("Public"),
		presentation: z.literal(true),
		type: z.literal("SlotMap"),
		content: structSchema.and(
			z.object({
				pos: fieldSchema
					.and(
						z.object({
							netVisibility: z.literal("Public"),
							presentation: z.literal(true),
							type: z.literal("Vec3"),
						}),
					)
					.optional(),
				rot: fieldSchema
					.and(
						z.object({
							netVisibility: z.literal("Public"),
							presentation: z.literal(true),
							type: z.literal("Quat"),
						}),
					)
					.optional(),
				scl: fieldSchema
					.and(
						z.object({
							netVisibility: z.literal("Public"),
							presentation: z.literal(true),
							type: z.literal("Vec3"),
						}),
					)
					.optional(),
			}),
		),

		entity: z.literal(true),
	}),
);

export type EntitySlotMap = z.infer<typeof entitySlotMapSchema>;

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
	//the simulationStateSchema is the top level structSchema but with special rules:
	.and(
		z.object({
			clients: clientsSlotMapSchema, //must contain client state
		}),
	)
	.and(
		z.record(
			z.string(),
			z.union([
				entitySlotMapSchema, //special entity structs with pos/rot/mat/interpolation
				fieldSchema.and(z.object({ entity: z.literal(false).optional() })), //block anything else from claiming that it's an entity
			]),
		),
	)
	//extra constraints not enforceable through zod's standard api/typescript
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) => path[0] !== "clients" && child.netVisibility === "Owner",
			error: (path) => `"Owner" visibility used outside of clients for "${path.join(".")}"`,
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
					child.netVisibility !== "Untracked"
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
				path[0] === "clients" && path[1] === "input" && child.netVisibility !== "Owner",
			error: (path, child) =>
				`Client input state's net visibility "${child.netVisibility}" for "${path.join(".")}" must be changed to "Owner"`,
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
				`In order to use presentation: true on "${path.join(".")}", its parent must also have presentation: true`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) =>
				child.type === "HapticPredictionEmitter" &&
				(child.netVisibility === "Untracked" || child.netVisibility === "Private"),
			//this is not a hard technical requirement, but it makes no sense
			//to use haptic predictions in this manner
			error: (path) =>
				`HapticPredictionEmitter at "${path.join(".")}" cannot have Untracked or Private netVisibility`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child, parent) =>
				parent?.type === "HapticPredictionEmitter" &&
				(child.netVisibility !== "Untracked" || !child.presentation),
			error: (path) =>
				`HapticPredictionEmitter field at "${path.join(".")}" must use { netVisibility: "Untracked, presentation: true }"`,
		}),
	);

export type SimulationState = z.infer<typeof simulationStateSchema>;

export function validate(state: unknown) {
	return simulationStateSchema.parse(state);
}
