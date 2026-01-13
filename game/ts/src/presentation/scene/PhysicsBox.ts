import { BoxGeometry, Mesh, MeshLambertMaterial } from "three";
import type { Object3D } from "three";

const characterGeom = new BoxGeometry();
const characterMat = new MeshLambertMaterial({ color: 0xffff00 });

export function spawnPhysicsBox() {
	return new Mesh(characterGeom, characterMat);
}

export function disposePhysicsBox(_box: Object3D): true {
	return true;
}
