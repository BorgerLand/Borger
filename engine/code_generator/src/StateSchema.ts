import { z } from "zod";
import type { DeeplyPartial } from "@engine/code_generator/Common.ts";

//true single-field primitives that can be easily read
//directly from wasm memory
export const simplePrimitives = [
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
] as const;

export const multiFieldPrimitives = [
	"Vec2", //xy, f32
	"DVec2", //xy, f64
	"Vec3A", //xyz, f32 (technically 4 floats for simd purposes, and 1 goes to waste)
	"DVec3", //xyz, f64
	"Quat", //xyzw, f32
	"DQuat", //xyzw, f64
] as const;

export const primitiveTypeSchema = z.enum([...simplePrimitives, ...multiFieldPrimitives]);

export const collectionTypeSchema = z.enum(["SlotMap"]);
export const utilityTypeSchema = z.enum([]);

export const NET_VISIBILITY_DEFAULT = "Private";
export const netVisibilitySchema = z.enum([
	NET_VISIBILITY_DEFAULT, //only server can access
	"Owner", //only server and the owning client can access
	"Public", //everyone can access
	"Untracked", //disable networking/diff tracking
]); //in order from least to most to make comparisons easier

export type NetVisibility = z.infer<typeof netVisibilitySchema>;
export type PrimitiveType = z.infer<typeof primitiveTypeSchema>;
export type CollectionType = z.infer<typeof collectionTypeSchema>;
export type UtilityType = z.infer<typeof utilityTypeSchema>;

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
						type: z.union([primitiveTypeSchema, utilityTypeSchema]),
						typeName: z.never().optional(),
						content: z.never().optional(),
					}),
					z.object({
						type: z.literal("struct"),
						typeName: z.string().optional(),
						content: structSchema,
					}),
					z.object({
						type: collectionTypeSchema,
						typeName: z.string().optional(),
						content: z.union([primitiveTypeSchema, utilityTypeSchema, structSchema]),
					}),
				]),
			),
		z.object({
			netVisibility: z.literal("Untracked"),
			presentation: z.boolean().optional(),
			type: z.string(),
			typeName: z.never().optional(),
			content: z.never().optional(),
		}),
	]),
) as z.ZodType<Field>;

//need to manually specify field's type because
//z.infer doesn't work on recursive structures
export type Field =
	| ((
			| {
					netVisibility?: Exclude<NetVisibility, "Untracked">;
					presentation?: false;
			  }
			| {
					netVisibility: Exclude<NetVisibility, "Private" | "Untracked">;
					presentation: true;
			  }
	  ) &
			(
				| {
						type: PrimitiveType | UtilityType;
						typeName?: never;
						content?: never;
				  }
				| {
						type: "struct";
						typeName?: string;
						content: Struct;
				  }
				| {
						type: CollectionType;
						typeName?: string;
						content: PrimitiveType | UtilityType | Struct;
				  }
			))
	| {
			netVisibility: "Untracked";
			presentation?: boolean;

			//any type that is Debug+Default should be fine.
			//alternatively, can be Debug+UntrackedState + contain
			//a `pub fn default() -> Self` method not associated with
			//the Default trait. if presentation: true, must also be
			//Clone. must specify fully qualified name if not a
			//primitive/utility/collection type. generics <> not
			//allowed; use a type alias instead
			type: string;

			typeName?: never;
			content?: never;
	  };

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
							type: z.literal("Vec3A"),
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
							type: z.literal("Vec3A"),
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
	test: (path: string[], child: Field, parent?: Field) => boolean; //false on fail
	error: (path: string[], child: Field, parent?: Field) => string;
}) {
	return traverse(struct);
	function traverse(childStruct: Struct, parentField?: Field, parentPath: string[] = []): boolean {
		for (const [childFieldName, childField] of Object.entries(childStruct)) {
			const childPath = [...parentPath, childFieldName];

			if (!test(childPath, childField, parentField)) {
				//eslint-disable-next-line no-console
				console.error(error(childPath, childField, parentField));
				return false;
			}

			if (
				childField.type === "struct" ||
				((collectionTypeSchema.options as string[]).includes(childField.type) &&
					typeof childField.content === "object")
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
			test: (path, child) => path[0] === "clients" || child.netVisibility !== "Owner",
			error: (path) => `"Owner" visibility used outside of clients for "${path.join(".")}"`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: function (path, child, parent) {
				if (!parent) return true;

				const hierarchy = netVisibilitySchema.options;
				return (
					hierarchy.indexOf(child.netVisibility ?? NET_VISIBILITY_DEFAULT) <=
						hierarchy.indexOf(parent.netVisibility ?? NET_VISIBILITY_DEFAULT) ||
					child.netVisibility === "Untracked"
				);
			},
			error: (path, child, parent) =>
				`Net visibility "${child.netVisibility ?? NET_VISIBILITY_DEFAULT}" for "${path.join(".")}" is more permissive than parent "${parent!.netVisibility ?? NET_VISIBILITY_DEFAULT}"`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) =>
				path[0] !== "clients" || path[1] !== "input" || child.netVisibility === "Owner",
			error: (path, child) =>
				`Client input state's net visibility "${child.netVisibility ?? NET_VISIBILITY_DEFAULT}" for "${path.join(".")}" must be changed to "Owner"`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) =>
				!(
					path[0] === "clients" &&
					path[1] === "input" &&
					((collectionTypeSchema.options as string[]).includes(child.type) ||
						(utilityTypeSchema.options as string[]).includes(child.type))
				),
			error: (path, child) =>
				`Client input state's type "${child.type}" for "${path.join(".")}" can't be a utility or collection type`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: (path, child) => !(path[0] === "clients" && path[1] === "input" && child.presentation),
			error: (path) => `Client input state "${path.join(".")}" can't use presentation: true`,
		}),
	)
	.refine((state) =>
		validateRecursively({
			struct: state,
			test: function (path, child, parent) {
				if (child.presentation && parent && !parent.presentation) return false;
				return true;
			},
			error: (path) =>
				`In order to use presentation: true on "${path.join(".")}", its parent must also have presentation: true`,
		}),
	);

export type SimulationState = z.infer<typeof simulationStateSchema>;
export type SimulationStateMod = DeeplyPartial<SimulationState>;

export function validate(state: unknown) {
	return simulationStateSchema.parse(state);
}
