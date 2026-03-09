import { init } from "@engine/client_ts/presentation_controller.ts";
import * as Pipeline from "@game/presentation/old/pipeline.ts";
import * as Entities from "@game/presentation/old/entities.ts";
import * as UI from "@game/presentation/ui/index.tsx";
import * as Crosshair from "@game/presentation/old/crosshair.ts";
import * as Input from "@game/presentation/input.ts";
import { AmbientLight, BoxGeometry, Color, DirectionalLight, Mesh } from "three";

Crosshair.init();

const engine = await init({
	canvasPromise: UI.init(),
	onPresentationTick: Pipeline.presentationTick,
	onSpawnEntity: Entities.spawnEntity,
	onDisposeEntity: Entities.disposeEntity,
	onResolutionChange: Crosshair.onResolutionChange,
	onDisconnect: Input.dispose,
});

Input.init(engine.renderer.renderer.domElement, engine.rsInput, engine.compat.Touchscreen.supported);

engine.renderer.renderer.setClearColor(new Color(0));
const scene = engine.renderer.scene3D;

scene.add(new AmbientLight(0xfff5e0, 1));

const directionalLight = new DirectionalLight(0xffffff, 3);
directionalLight.position.set(1, 1, 1);
scene.add(directionalLight);

//point of reference
const box = new Mesh(new BoxGeometry());
box.position.set(0, 0, -5);
scene.add(box);
