import type { SimulationState } from "@engine/code_generator/StateSchema.ts";

//make sure field names are snake_case or else you will anger rustc
export default {
	clients: {
		netVisibility: "Public",
		presentation: true,
		type: "SlotMap",
		typeName: "ClientState",
		content: {
			input: {
				netVisibility: "Owner",
				type: "struct",
				typeName: "InputState",
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
					cam_yaw: { netVisibility: "Owner", type: "f32" }, //horizontal,
					cam_pitch: { netVisibility: "Owner", type: "f32" }, //vertical
					cam_radius: { netVisibility: "Owner", type: "f32" }, //orbit distance from eyeballs

					//omnidirectional movement - 3D analog stick
					//x = left/right, y = down/up, z = forward/back
					//all axes in range [-1, 1]
					omnidir: { netVisibility: "Owner", type: "Vec3" },
				},
			},

			id: { netVisibility: "Public", presentation: true, type: "usize32" }, //the corresponding entity id
		},
	},
	characters: {
		netVisibility: "Public",
		presentation: true,
		entity: true,
		type: "SlotMap",
		typeName: "Character",
		content: {
			pos: { netVisibility: "Public", presentation: true, type: "Vec3" },
			rot: { netVisibility: "Public", presentation: true, type: "Quat" },
		},
	},
} satisfies SimulationState;
