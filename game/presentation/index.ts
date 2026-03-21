import * as Borger from "@borger/ts";
import * as UI from "@game/presentation/ui/index.tsx";
import * as Input from "@game/presentation/input.ts";
import { WebGPURenderer } from "three/webgpu";
import * as Character from "@game/presentation/character.ts";
import { AmbientLight, BoxGeometry, Color, DirectionalLight, Mesh, PerspectiveCamera, Scene } from "three";

async function game(engine: Borger.Init) {
	const canvas = await UI.init();
	Input.init(canvas, engine.compat.Touchscreen.supported);

	const renderer = new WebGPURenderer({
		canvas,
		powerPreference: "high-performance",
		antialias: true,
	});

	await renderer.init();
	renderer.setClearColor(new Color(0));
	const scene = new Scene();
	const camera = new PerspectiveCamera(67);

	window.onresize = onresize;
	onresize();
	function onresize() {
		renderer.setSize(innerWidth, innerHeight);
		camera.aspect = innerWidth / innerHeight;
		camera.updateProjectionMatrix();
	}

	const directionalLight = new DirectionalLight(0xffffff, 3);
	directionalLight.position.set(1, 1, 1);
	scene.add(directionalLight);
	scene.add(new AmbientLight(0xfff5e0, 1));

	//point of reference
	const box = new Mesh(new BoxGeometry());
	box.position.set(0, 0, -5);
	scene.add(box);

	//purely client sided rendering pipeline. it should
	//be able to able to render the game in any state,
	//regardless of what the simulation is doing. remember
	//that rollbacks/mispredicts can wipe out data that
	//was already rendered in a previous frame
	return function presentationLoop(input: Borger.Input, output: Borger.Output) {
		//populate input state from poll
		Input.update(input);

		Character.update(input, output, scene, camera);

		//this line of code might have something to do with rendering
		renderer.render(scene, camera);
	};
}

await Borger.init({ game });

Input.dispose();
