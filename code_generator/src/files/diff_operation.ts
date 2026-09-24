import { ENGINE_GENERATED_DIR } from "@borger/code_generator/common.ts";
import { writeFileSync } from "fs";
import type { FlattenedOutput } from "@borger/code_generator/flatten.ts";

export function generateDiffOperation(flattened: FlattenedOutput) {
	let curID = 0;
	writeFileSync(
		`${ENGINE_GENERATED_DIR}/diff_operation.json`,
		`{
	"base":
	{
		"SetPrimitive": ${curID++},
		
		${/*path navigation*/ ""}
		"NavigateUp": ${curID++},${/*   unix analogy: "cd ../../.." go to parent directory*/ ""}
		"NavigateDown": ${curID++},${/* unix analogy: "cd x/y/z" open directory*/ ""}
		"NavigateReset": ${curID++},${/*unix analogy: "cd /" go to root directory*/ ""}
		
		${
			/*(rollback only) insert a wall between ticks in
		order to know when to stop rolling back a tick.
		tx system sends packet size in bytes instead*/ ""
		}
		"RollbackTickSeparator": ${curID++}
	},
	"SlotMap":
	{
		"SlotMapAdd": ${curID++},
		"SlotMapRemove": ${curID++},
		"SlotMapClear": ${curID++}
	}${flattened.plugins
		.map(
			(plugin) => `,
	"${plugin.name}":
	{
${plugin.diffOps.map((diffOp) => `		"${diffOp}": ${curID++}`).join(`,
`)}
	}`,
		)
		.join("")}
}`,
	);
}
