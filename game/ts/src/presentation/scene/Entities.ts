import * as ClientRS from "@engine/client_rs";
import type { Object3D } from "three";
import { spawnCharacter, disposeCharacter } from "@presentation/scene/Character.ts";
import { spawnPhysicsBox, disposePhysicsBox } from "@presentation/scene/PhysicsBox.ts";

export function spawnEntity(type: ClientRS.EntityKind): Object3D {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return spawnCharacter();
		case ClientRS.EntityKind.PhysicsBox:
			return spawnPhysicsBox();
	}
}

export function disposeEntity(type: ClientRS.EntityKind, entity: Object3D): true {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return disposeCharacter(entity);
		case ClientRS.EntityKind.PhysicsBox:
			return disposePhysicsBox(entity);
	}
}
