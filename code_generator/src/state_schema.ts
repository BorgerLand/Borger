import { z, type util } from "zod";
import type {
	NetVisibility,
	RecursiveSchemaValidator,
	Field,
	Struct,
	Presentation,
	Plugin,
} from "@borger/plugin_sdk";

const identifierSchema = z.string().regex(/^[a-zA-Z_][a-zA-Z0-9_]*$/);

const netVisibilitySchema = z.enum([
	//in order from least to most to make comparisons easier
	"private", //only server can access
	"owner", //only server and the owning client can access
	"public", //everyone can access
	"untracked", //disable networking/diff tracking
]) satisfies z.ZodEnum<util.ToEnum<NetVisibility>> & z.ZodType<NetVisibility>;

const presentationSchema = z.enum(["clone", "interpolate"]) satisfies z.ZodEnum<util.ToEnum<Presentation>> &
	z.ZodType<Presentation>;

//- primitive
//	- simple primitive (some are interpolable)
//	- multi-field primitive (some are interpolable)
//- collection (currently just built in slotmap type)
//- plugin
//	- tracked
//	- untracked

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
] satisfies PrimitiveType[]);

export const primitiveTypeSchema = z.enum([
	...simplePrimitiveTypeSchema.options,
	...multiFieldPrimitiveTypeSchema.options,
]);
export const collectionTypeSchema = z.enum(["SlotMap"]);

export type InterpolablePrimitiveType = z.infer<typeof interpolablePrimitiveTypeSchema>;
export type SimplePrimitiveType = z.infer<typeof simplePrimitiveTypeSchema>;
export type PrimitiveType = z.infer<typeof primitiveTypeSchema>;
export type MultiFieldPrimitiveType = z.infer<typeof multiFieldPrimitiveTypeSchema>;
export type CollectionType = z.infer<typeof collectionTypeSchema>;

export function stateSchema<TrackedPluginType extends string, UntrackedPluginType extends string>(
	trackedPluginTypeSchema: z.ZodEnum<util.ToEnum<TrackedPluginType>>,
	untrackedPluginTypeSchema: z.ZodEnum<util.ToEnum<UntrackedPluginType>>,
	plugins: Plugin[],
) {
	//must specify type because of recursion, and can't use record<> for some reason
	const structSchema: z.ZodType<{
		[fieldName: string]: z.infer<typeof fieldSchema>;
	}> = z.record(
		identifierSchema,
		z.lazy(() => fieldSchema),
	);

	const fieldSchema = z.union([
		//NETWORKED
		z
			.union([
				z.object({
					netVisibility: netVisibilitySchema.exclude(["untracked"]),
					presentation: z.never().optional(),
				}),
				z.object({
					//disallow presentation on private state
					netVisibility: netVisibilitySchema.exclude(["private", "untracked"]),
					presentation: presentationSchema.optional(),
				}),
			])
			.and(
				z.union([
					z.object({
						type: interpolablePrimitiveTypeSchema,
						typeName: z.never().optional(),
						content: z.never().optional(),
						presentation: presentationSchema.optional(),
					}),
					z.object({
						type: z.union([primitiveTypeSchema, trackedPluginTypeSchema]),
						typeName: z.never().optional(),
						content: z.never().optional(),
						presentation: z.literal("clone").optional(),
					}),
					z.object({
						type: z.literal("struct"),
						typeName: identifierSchema.optional(),
						content: structSchema,
						presentation: z.literal("clone").optional(),
					}),
					z.object({
						type: collectionTypeSchema,
						typeName: identifierSchema.optional(),
						//collection doesn't work here because there would be no way
						//of answering "collection of what?". untracked plugin would
						//make no sense because it would require tracking existence of
						//slots but with no tracked data in them. however, using the
						//struct type would allow them as fields
						content: z.union([structSchema, primitiveTypeSchema, trackedPluginTypeSchema]),
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
						presentation: presentationSchema.optional(),
					}),
					z.object({
						type: z.union([primitiveTypeSchema, untrackedPluginTypeSchema]),
						presentation: z.literal("clone"),
					}),
					z.object({
						//- must specify fully qualified name (including the leading
						//::)
						//- chosen type must either be Debug+Default OR Debug+
						//UntrackedState+contain a `pub(crate) fn default() -> Self`
						//method not associated with the Default trait.
						type: z.string(), //referred to as "external" type kind by flattener
						presentation: z.never().optional(),
					}),
				]),
			),
	]);

	let stateSchema = structSchema
		.and(
			//must contain client state
			z.object({
				clients: fieldSchema.and(
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
				),
			}),
		)
		//extra constraints not enforceable through zod's standard api/typescript
		.refine((state) =>
			validateRecursively(state, {
				isInvalid: (path, child) => path[0] !== "clients" && child.netVisibility === "owner",
				error: (path) => `"owner" visibility used outside of clients for "${path.join(".")}"`,
			}),
		)
		.refine((state) =>
			validateRecursively(state, {
				isInvalid: function (path, child, parent) {
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
			validateRecursively(state, {
				isInvalid: (path, child) =>
					path[0] === "clients" && path[1] === "input" && child.netVisibility !== "owner",
				error: (path, child) =>
					`Client input state's net visibility "${child.netVisibility}" for "${path.join(".")}" must be changed to "owner"`,
			}),
		)
		.refine((state) =>
			validateRecursively(state, {
				isInvalid: (path, child) =>
					path[0] === "clients" &&
					path[1] === "input" &&
					((collectionTypeSchema.options as string[]).includes(child.type) ||
						(untrackedPluginTypeSchema.options as string[]).includes(child.type) ||
						(trackedPluginTypeSchema.options as string[]).includes(child.type)),
				error: (path, child) =>
					`Client input state's type "${child.type}" for "${path.join(".")}" can't be a plugin or collection type`,
			}),
		)
		.refine((state) =>
			validateRecursively(state, {
				isInvalid: (path, child) =>
					path[0] === "clients" && path[1] === "input" && Boolean(child.presentation),
				error: (path) => `Client input state "${path.join(".")}" can't use presentation: true`,
			}),
		)
		.refine((state) =>
			validateRecursively(state, {
				isInvalid: (path, child, parent) =>
					Boolean(child.presentation && parent && !parent.presentation),
				error: (path) =>
					`In order to enable presentation on "${path.join(".")}", its parent must also have presentation enabled`,
			}),
		)
		.refine((state) =>
			//this is not caught by tsc because external type string can be anything
			validateRecursively(state, {
				isInvalid: (path, child) =>
					child.netVisibility === "untracked" &&
					((collectionTypeSchema.options as string[]).includes(child.type) ||
						(trackedPluginTypeSchema.options as string[]).includes(child.type)),
				error: (path, child) =>
					`Untracked field of type "${child.type}" for "${path.join(".")}" can't be a tracked plugin or collection type`,
			}),
		);

	for (const plugin of plugins)
		if (plugin.schemaValidators)
			for (const validator of plugin.schemaValidators)
				stateSchema = stateSchema.refine((state) => validateRecursively(state, validator));

	return stateSchema;
}

export type State<TrackedPluginType extends string, UntrackedPluginType extends string> = z.infer<
	ReturnType<typeof stateSchema<TrackedPluginType, UntrackedPluginType>>
>;

function validateRecursively(struct: Struct, { isInvalid, error }: RecursiveSchemaValidator) {
	return traverse(struct);
	function traverse(childStruct: Struct, parentField?: Field, parentPath: string[] = []): boolean {
		for (const [childFieldName, childField] of Object.entries(childStruct)) {
			const childPath = [...parentPath, childFieldName];

			if (isInvalid(childPath, childField, parentField)) {
				//eslint-disable-next-line no-console
				console.error(error(childPath, childField, parentField));
				return false;
			}

			if (typeof childField.content === "object") {
				if (!traverse(childField.content as Struct, childField, childPath)) {
					return false;
				}
			}
		}

		return true;
	}
}
