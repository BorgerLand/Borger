import type { RendererState } from "@engine/client_ts/Renderer.ts";
import { Sprite, SpriteMaterial, TextureLoader } from "three";

let spr: Sprite;
let loadPromise: Promise<void>;

export function init() {
	loadPromise = new Promise(function (resolve) {
		const tex = new TextureLoader().load("/crosshair.webp", function () {
			spr.scale.set(tex.image.width, tex.image.height, 1);
			resolve();
		});

		const mat = new SpriteMaterial({ map: tex, depthTest: false, depthWrite: false, transparent: true });
		spr = new Sprite(mat);
		spr.center.set(0, 0);
		spr.matrixWorld = spr.matrix;
		spr.frustumCulled = false;
	});
}

export async function onResolutionChange(state: RendererState) {
	await loadPromise;

	state.scene2D.add(spr);
	spr.position.x = Math.ceil((state.renderer.domElement.width - spr.scale.x) / 2);
	spr.position.y = Math.ceil((state.renderer.domElement.height - spr.scale.y) / 2);
	spr.updateMatrix();
}
