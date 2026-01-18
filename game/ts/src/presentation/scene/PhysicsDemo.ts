import { BoxGeometry, Mesh, MeshLambertMaterial, SphereGeometry } from "three";
import type { Object3D } from "three";

const cubeGeom = new BoxGeometry();
const cubeMat = new MeshLambertMaterial({ color: 0xffff00 });
const sphereGeom = new SphereGeometry(0.5);
const sphereMat = new MeshLambertMaterial();

export function spawnCube() {
	return new Mesh(cubeGeom, cubeMat);
}

export function disposeCube(_box: Object3D): true {
	return true;
}

export function spawnSphere() {
	return new Mesh(sphereGeom, sphereMat);
}

export function disposeSphere(_box: Object3D): true {
	return true;
}
