import type { SimulationState } from "@borger/code_generator/state_schema.ts";

//make sure field names are snake_case or else you will anger rustc
export default {
	clients: {
		netVisibility: "public",
		presentation: "clone",
		type: "SlotMap",
		typeName: "Client",
		content: {
			input: {
				netVisibility: "owner",
				type: "struct",
				typeName: "Input",
				content: {
					//inputs should represent REQUESTS to perform ACTIONS,
					//not the specific buttons/combos that trigger them,
					//because different platforms require triggering the
					//same gameplay action with different controls. also
					//keep in mind client-sided stuff like "mute" or "open
					//inventory" generally don't belong here. only inputs
					//that affect the multiplayer simulation should be
					//listed here. input.rs must be updated accordingly

					//the camera's target spherical coordinate
					cam_yaw: { netVisibility: "owner", type: "f32" }, //horizontal,
					cam_pitch: { netVisibility: "owner", type: "f32" }, //vertical

					//omnidirectional movement - 3D analog stick
					//x = left/right, y = down/up, z = forward/back
					//all axes in range [-1, 1]
					omnidir: { netVisibility: "owner", type: "Vec3" },
				},
			},

			character_id: { netVisibility: "public", presentation: "clone", type: "usize32" },
		},
	},
	characters: {
		netVisibility: "public",
		presentation: "clone",
		type: "SlotMap",
		typeName: "Character",
		content: {
			pos: { netVisibility: "public", presentation: "interpolate", type: "Vec3" },
			rot: { netVisibility: "public", presentation: "interpolate", type: "Quat" },
		},
	},
} satisfies SimulationState;
