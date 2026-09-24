import type { Plugin } from "@borger/plugin_sdk";

export const eventDispatcher = {
	name: "EventDispatcher",
	tracked: true,
	schemaValidators: [
		{
			isInvalid: (path, child) => child.type === "EventDispatcher" && !child.presentation,
			//this is not a hard technical requirement, but it makes no sense
			//to use event dispathcer in this manner
			error: (path) => `EventDispatcher at "${path.join(".")}" must have presentation enabled`,
		},
	],
	rustSimFQN: "::borger_event_dispatcher::EventDispatcher",
	nodePackageName: "@borger/event_dispatcher",
	diffOps: ["EventDispatcher"],
	tsMemWrappers: (offset) => `state.memView.getUint8(ptr + ${offset}) !== 0`,
} satisfies Plugin<"EventDispatcher">;
