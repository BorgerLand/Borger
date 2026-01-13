import { init } from "@engine/client_ts/PresentationController.ts";
import * as Pipeline from "@presentation/scene/Pipeline.ts";
import * as Entities from "@presentation/scene/Entities.ts";
import * as UI from "@presentation/ui/Index.tsx";
import * as Crosshair from "@presentation/scene/Crosshair.ts";
import * as Input from "@simulation/Input.ts";
import { Color, DirectionalLight, LightProbe, SphericalHarmonics3, Vector3 } from "three";

Crosshair.init();

const engine = await init({
	canvasPromise: UI.init(),
	onPresentationTick: Pipeline.presentationTick,
	onSpawnEntity: Entities.spawnEntity,
	onDisposeEntity: Entities.disposeEntity,
	onResolutionChange: Crosshair.onResolutionChange,
	onDisconnect: Input.dispose,
});

Input.init(engine.renderer.renderer.domElement, engine.rsInput);

engine.renderer.renderer.setClearColor(new Color(0));
const scene = engine.renderer.scene3D;

//values come from https://github.com/mrdoob/three.js/tree/master/examples/textures/cube/Bridge2
//this was adding 3+ solid seconds to load time so just hardcode the result
const sh = new SphericalHarmonics3();
sh.coefficients = [
	new Vector3(0.30321116464819775, 0.469139033091744, 0.6612119046057533),
	new Vector3(0.08308781194826696, 0.1738950506966252, 0.34496710972023753),
	new Vector3(0.12151515265584699, 0.10355380691012983, 0.06522164816228027),
	new Vector3(0.013589948387895873, 0.019171346568635082, 0.018685627782300207),
	new Vector3(0.01080149563645966, 0.01576676387507492, 0.017422775760709042),
	new Vector3(0.06305945908273995, 0.06102245270043295, 0.04428108645549519),
	new Vector3(0.19942673441776695, 0.22011240833921147, 0.19460683653331134),
	new Vector3(0.019619214656099664, 0.032428373426532764, 0.043528643877490614),
	new Vector3(0.1329904929776519, 0.19633539747173717, 0.2189671701645584),
];
scene.add(new LightProbe(sh, 2));

const directionalLight = new DirectionalLight(0xffffff, 3);
directionalLight.position.set(1, 1, 1);
scene.add(directionalLight);
