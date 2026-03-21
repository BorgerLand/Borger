/* eslint-disable no-console */

import state from "../../../game/state.ts";

import { validate } from "@borger/code_generator/state_schema.ts";
import { flatten } from "@borger/code_generator/flatten.ts";

import { generateSimulationState } from "@borger/code_generator/files/simulation_state.ts";
import { generateConstructors } from "@borger/code_generator/files/constructors.ts";
import { generateSnapshotSerDes } from "@borger/code_generator/files/snapshot_serdes.ts";
import { generateDiffSer } from "@borger/code_generator/files/diff_ser.ts";
import { generateDiffDes } from "@borger/code_generator/files/diff_des.ts";
import { generateUntracked } from "@borger/code_generator/files/untracked.ts";
import { generatePresentation } from "@borger/code_generator/files/presentation.ts";
import { generateInterpolation } from "@borger/code_generator/files/interpolation.ts";
import { generateMemOffsets } from "@borger/code_generator/files/mem_offsets.ts";
import { generateMemWrappers } from "@borger/code_generator/files/mem_wrappers.ts";
console.time("Great success");

let validState;
try {
	validState = validate(state);
} catch (oops) {
	//the complex schema emits laughably illegible type errors,
	//so just let tsc's error printing system do the job
	if (String(oops).length > 5000) throw Error("Type error in state.ts (see output of TSC CHECK)");
	else throw oops;
}

const structs = flatten(validState);
generateSimulationState(structs);
generateConstructors(structs.sim);
generateSnapshotSerDes(structs.sim);
generateDiffSer(structs);
generateDiffDes(structs);
generateUntracked(structs.sim);
generatePresentation(structs.sim);
generateInterpolation(structs.sim);
generateMemOffsets(structs);
generateMemWrappers(structs);

console.timeEnd("Great success");
