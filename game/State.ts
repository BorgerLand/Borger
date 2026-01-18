import type { EntitySlotMap, SimulationState } from "@engine/code_generator/StateSchema.ts";

//make sure field names are snake_case or else you will anger rustc
export default {
	physics: { netVisibility: "Untracked", type: "crate::physics::Physics" },
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

					//omnidirectional movement - 2D analog stick
					//x = left/right, y = forward/back
					//all axes in range [-1, 1]
					omnidir: { netVisibility: "Owner", type: "Vec2" },
					jumping: { netVisibility: "Owner", type: "bool" },

					start_physics_test: { netVisibility: "Owner", type: "bool" },
					blow_nose: { netVisibility: "Owner", type: "bool" },
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
			prv_pos: { netVisibility: "Public", presentation: true, type: "Vec3A" },
			pos: { netVisibility: "Public", presentation: true, type: "Vec3A" },
			rot: { netVisibility: "Public", presentation: true, type: "Quat" },
			velocity: { netVisibility: "Public", type: "Vec3A" },
			grounded: { netVisibility: "Public", type: "bool" },
		},
	},
	running_physics_test: { netVisibility: "Public", type: "bool" },
	cubes: rigidBody("PhysicsCube"),
	spheres: rigidBody("PhysicsSphere"),
} satisfies SimulationState;

function rigidBody(typeName: string): EntitySlotMap {
	return {
		netVisibility: "Public",
		presentation: true,
		entity: true,
		type: "SlotMap",
		typeName,
		content: {
			pos: { netVisibility: "Public", presentation: true, type: "Vec3A" },
			rot: { netVisibility: "Public", presentation: true, type: "Quat" },
			linvel: { netVisibility: "Public", type: "Vec3A" },
			angvel: { netVisibility: "Public", type: "Vec3A" },
			sleeping: { netVisibility: "Public", type: "bool" },
			time_since_can_sleep: { netVisibility: "Public", type: "f32" },
			rb_handle: {
				netVisibility: "Untracked",
				type: "rapier3d::prelude::RigidBodyHandle",
			},
		},
	};
}
