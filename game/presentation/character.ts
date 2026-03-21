import type * as Borger from "@borger/ts";
import { ConeGeometry, MathUtils, Mesh, MeshLambertMaterial, type Camera, type Scene } from "three";

const charactersPres = new Map<number, Mesh>();

const characterGeom = new ConeGeometry();
characterGeom.rotateX(-90 * MathUtils.DEG2RAD);
characterGeom.scale(0.3, 0.3, 0.7);
const characterMat = new MeshLambertMaterial({ color: 0x00ff00 });
const characterMesh = new Mesh(characterGeom, characterMat);

export function update(input: Borger.Input, output: Borger.Output, scene: Scene, camera: Camera) {
	const localCharacterID = output.state.clients().get(output.local_client_id)!.value.character_id;

	const charactersSim = output.state.characters({
		added(id) {
			const characterPres = characterMesh.clone();
			charactersPres.set(id, characterPres);
			scene.add(characterPres);
		},

		removed(id) {
			const characterPres = charactersPres.get(id)!;
			characterPres.removeFromParent();
			charactersPres.delete(id);
		},
	});

	for (const [id, characterSim] of charactersSim) {
		const mesh = charactersPres.get(id)!;
		if (localCharacterID === id) {
			mesh.visible = false;
			camera.position.copy(characterSim.pos);

			//players' toleration for latency between moving the mouse and seeing camera
			//movement is so extremely low that not even the Immediate multiplayer tradeoff
			//is fast enough due to the rtt of the presentation thread sending the camera
			//input, receiving a response, and interpolating towards it. so, cheat here by
			//by directly writing the latest input state to camera rotation. there is no risk
			//of mispredicting because inputs are client authoritative
			camera.rotation.set(input.get_cam_pitch(), input.get_cam_yaw(), 0, "ZYX");
		} else {
			mesh.visible = true;
			mesh.position.copy(characterSim.pos);
			mesh.quaternion.copy(characterSim.rot);
		}
	}
}
