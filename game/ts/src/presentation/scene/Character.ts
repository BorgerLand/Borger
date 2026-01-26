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

//values copied from rust
/*const colliderHeight = 2.2;
const colliderRadius = 0.35;
const debugGeom = new CapsuleGeometry(colliderRadius, colliderHeight - 2 * colliderRadius);
const debugMat = new MeshBasicMaterial();
debugMat.wireframe = true;
const debugMesh = new Mesh(debugGeom, debugMat);*/

export function spawnCharacter() {
	const entity = new Object3D();
	gruPromise.then((gru) => entity.add(gru.clone()));
	return entity;
}

export function disposeCharacter(_character: Object3D): true {
	return true;
}
