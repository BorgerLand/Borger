import * as ClientRS from "@engine/client_rs";
import type { Object3D } from "three";
import * as Character from "@game/presentation/scene/character.ts";

export function spawnEntity(type: ClientRS.EntityKind, _id: number): Object3D {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return Character.spawn();
	}
}

export function disposeEntity(type: ClientRS.EntityKind, entity: Object3D, _id: number): true {
	switch (type) {
		case ClientRS.EntityKind.Character:
			return Character.dispose(entity);
	}
}
