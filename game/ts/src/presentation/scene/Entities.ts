import * as ClientRS from "@engine/client_rs";
import type { Object3D } from "three";
import * as Character from "@presentation/scene/Character.ts";

export function spawnEntity(type: ClientRS.EntityKind): Object3D {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return Character.spawn();
	}
}

export function disposeEntity(type: ClientRS.EntityKind, entity: Object3D): true {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return Character.dispose(entity);
	}
}
