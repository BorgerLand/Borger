import type { Plugin } from "@borger/plugin_sdk";

export const rapier = {
	name: "Rapier",
	tracked: false,
	rsSimulationFQN: "::borger_rapier::Rapier",
	nodePackageName: "@borger/rapier",

	schemaValidators: [
		{
			isInvalid: (path, child) => child.type === "Rapier" && Boolean(child.presentation),
			error: (path) => `Rapier at "${path.join(".")}" must have presentation disabled`,
		},
	],
} satisfies Plugin<"Rapier">;
