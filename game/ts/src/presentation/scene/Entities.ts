import * as ClientRS from "@engine/client_rs";
import type { Object3D } from "three";
import { spawnCharacter, disposeCharacter } from "@presentation/scene/Character.ts";
import { spawnCube, disposeCube, spawnSphere, disposeSphere } from "@presentation/scene/PhysicsDemo.ts";

export function spawnEntity(type: ClientRS.EntityKind): Object3D {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return spawnCharacter();
		case ClientRS.EntityKind.PhysicsCube:
			return spawnCube();
		case ClientRS.EntityKind.PhysicsSphere:
			return spawnSphere();
	}
}

export function disposeEntity(type: ClientRS.EntityKind, entity: Object3D): true {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return disposeCharacter(entity);
		case ClientRS.EntityKind.PhysicsCube:
			return disposeCube(entity);
		case ClientRS.EntityKind.PhysicsSphere:
			return disposeSphere(entity);
	}
}
