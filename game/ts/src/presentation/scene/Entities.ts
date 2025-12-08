import * as ClientRS from "@engine/client_rs";
import type { Object3D } from "three";
import { spawnCharacter, disposeCharacter } from "@presentation/scene/Character.ts";

export function spawnEntity(type: ClientRS.EntityKind): Object3D {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return spawnCharacter();
	}
}

export function disposeEntity(type: ClientRS.EntityKind, entity: Object3D): true {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return disposeCharacter(entity);
	}
}
