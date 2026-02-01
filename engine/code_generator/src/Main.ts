/* eslint-disable no-console */

import state from "../../../game/State.ts";

import { validate } from "@engine/code_generator/StateSchema.ts";
import { flatten } from "@engine/code_generator/Flatten.ts";

import { generateSimulationStateRS } from "@engine/code_generator/files/simulation_state.ts";
import { generatePresentationStateRS } from "@engine/code_generator/files/presentation_state.ts";
import { generateConstructorsRS } from "@engine/code_generator/files/constructors.ts";
import { generateDiffSerRS } from "@engine/code_generator/files/diff_ser.ts";
import { generateDiffDesRS } from "@engine/code_generator/files/diff_des.ts";
import { generateSnapshotSerDesRS } from "@engine/code_generator/files/snapshot_serdes.ts";
import { generateUntracked } from "@engine/code_generator/files/untracked.ts";
import { generateInterpolationRS } from "@engine/code_generator/files/interpolation.ts";

console.time("Great success");

const structs = flatten(validate(state));
generateSimulationStateRS(structs);
generatePresentationStateRS(structs.sim);
generateConstructorsRS(structs.sim);
generateUntracked(structs.sim);
generateDiffSerRS(structs);
generateDiffDesRS(structs);
generateSnapshotSerDesRS(structs.sim);
generateInterpolationRS(structs.sim);

console.timeEnd("Great success");
