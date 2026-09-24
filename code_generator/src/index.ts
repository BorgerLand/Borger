/*eslint-disable no-console*/

import { stateSchema, type State } from "@borger/code_generator/state_schema.ts";
import z from "zod";
import { flatten } from "@borger/code_generator/flatten.ts";
import { mkdirSync, writeSync } from "fs";
import {
	ENGINE_GENERATED_DIR,
	CLIENT_RS_GENERATED_DIR,
	CLIENT_TS_GENERATED_DIR,
} from "@borger/code_generator/common.ts";
import type { Plugin } from "@borger/plugin_sdk";

import { generateDiffOperation } from "@borger/code_generator/files/diff_operation.ts";
import { generatePluginExports } from "@borger/code_generator/files/plugin_exports.ts";
import { generateEngineCargoTOML } from "@borger/code_generator/files/engine_cargo_toml.ts";
import { generateSimulation } from "@borger/code_generator/files/simulation.ts";
import { generateConstructors } from "@borger/code_generator/files/constructors.ts";
import { generateSnapshotSerDes } from "@borger/code_generator/files/snapshot_serdes.ts";
import { generateDiffSer } from "@borger/code_generator/files/diff_ser.ts";
import { generateDiffDes } from "@borger/code_generator/files/diff_des.ts";
import { generateUntracked } from "@borger/code_generator/files/untracked.ts";
import { generatePresentation } from "@borger/code_generator/files/presentation.ts";
import { generateInterpolation } from "@borger/code_generator/files/interpolation.ts";
import { generateMemOffsets } from "@borger/code_generator/files/mem_offsets.ts";
import { generateMemWrappers } from "@borger/code_generator/files/mem_wrappers.ts";

process.on("exit", function (code) {
	if (code === 0) console.timeEnd("Great success");
	else writeSync(2, `Code generation failed\n`);
});

console.time("Great success");

//generics allow typescript error highlighting to recognize plugin types in state.ts
type CodeGenerator<TrackedPluginType extends string = never, UntrackedPluginType extends string = never> = {
	plugin<T extends string>(
		type: Plugin<T> & { tracked: true },
	): CodeGenerator<TrackedPluginType | T, UntrackedPluginType>;

	plugin<T extends string>(
		type: Plugin<T> & { tracked: false },
	): CodeGenerator<TrackedPluginType, UntrackedPluginType | T>;

	flatulate(state: State<TrackedPluginType, UntrackedPluginType>): void;
};

export function codeGenerator<
	TrackedPluginType extends string = never,
	UntrackedPluginType extends string = never,
>(): CodeGenerator<TrackedPluginType, UntrackedPluginType> {
	const tracked: TrackedPluginType[] = [];
	const untracked: UntrackedPluginType[] = [];
	const plugins: Plugin[] = [];

	return {
		plugin(plugin: Plugin) {
			if (
				tracked.includes(plugin.name as TrackedPluginType) ||
				untracked.includes(plugin.name as UntrackedPluginType)
			)
				throw Error(`Found multiple plugins with type name "${plugin.name}"`);

			plugins.push(plugin);
			if (plugin.tracked) {
				tracked.push(plugin.name as TrackedPluginType);
			} else {
				untracked.push(plugin.name as UntrackedPluginType);
			}

			return this;
		},

		flatulate(state: State<TrackedPluginType, UntrackedPluginType>) {
			const stateSchemaInstance = stateSchema(z.enum(tracked), z.enum(untracked), plugins);

			let validState;
			try {
				validState = stateSchemaInstance.parse(state);
			} catch (oops) {
				//the complex schema emits laughably illegible type errors,
				//so just let tsc's error printing system do the job

				if (String(oops).length > 5000) {
					//eslint-disable-next-line preserve-caught-error
					throw Error("Type error in state.ts (see output of TSC-CODEGEN)");
				}

				throw oops;
			}

			const flattened = flatten(validState, plugins);
			mkdirSync(ENGINE_GENERATED_DIR, { recursive: true });
			mkdirSync(CLIENT_RS_GENERATED_DIR, { recursive: true });
			mkdirSync(CLIENT_TS_GENERATED_DIR, { recursive: true });
			generateDiffOperation(flattened);
			generatePluginExports(flattened);
			generateEngineCargoTOML(flattened);
			generateSimulation(flattened);
			generateConstructors(flattened);
			generateSnapshotSerDes(flattened);
			generateDiffSer(flattened);
			generateDiffDes(flattened);
			generateUntracked(flattened);
			generatePresentation(flattened);
			generateInterpolation(flattened);
			generateMemOffsets(flattened);
			generateMemWrappers(flattened);
		},
	};
}
