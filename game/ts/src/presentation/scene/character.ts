import { ConeGeometry, MathUtils, Mesh, MeshLambertMaterial } from "three";
import type { Object3D } from "three";

const characterGeom = new ConeGeometry();
characterGeom.rotateX(-90 * MathUtils.DEG2RAD);
characterGeom.scale(0.3, 0.3, 0.7);
const characterMat = new MeshLambertMaterial({ color: 0x00ff00 });

export function spawn() {
	return new Mesh(characterGeom, characterMat);
}

export function dispose(_character: Object3D): true {
	return true;
}
