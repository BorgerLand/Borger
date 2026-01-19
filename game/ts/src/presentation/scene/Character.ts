import { Object3D } from "three";
import { GLTFLoader } from "three/examples/jsm/Addons.js";

const loader = new GLTFLoader();
const gruPromise = loader.loadAsync("/gru.glb").then(function (result) {
	const gru = result.scene;
	gru.position.y -= 1.5;
	gru.rotation.y = Math.PI;
	gru.scale.setScalar(0.18);
	return gru;
});

export function spawnCharacter() {
	const entity = new Object3D();
	gruPromise.then((gru) => entity.add(gru.clone()));
	console.trace(entity);
	return entity;
}

export function disposeCharacter(_character: Object3D): true {
	return true;
}

/*import { CapsuleGeometry, ConeGeometry, MathUtils, Mesh, MeshLambertMaterial } from "three";
import type { Object3D } from "three";
import { BufferGeometryUtils } from "three/examples/jsm/Addons.js";

const RADIUS = 0.35;
const CYL_HEIGHT = 2.2;
const EYE_HEIGHT = 2.55;

const capsule = new CapsuleGeometry(RADIUS, CYL_HEIGHT);
const cone = new ConeGeometry();
cone.rotateX(-90 * MathUtils.DEG2RAD);
cone.scale(0.3, 0.3, 1.2);
cone.translate(0, EYE_HEIGHT - (RADIUS + CYL_HEIGHT / 2), -0.3);
const characterGeom = BufferGeometryUtils.mergeGeometries([capsule, cone]);
const characterMat = new MeshLambertMaterial({ color: 0x00ff00 });

export function spawnCharacter() {
	return new Mesh(characterGeom, characterMat);
}

export function disposeCharacter(_character: Object3D): true {
	return true;
}*/
