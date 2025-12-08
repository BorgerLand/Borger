import { PerspectiveCamera, Scene, Object3D, OrthographicCamera } from "three";
import { WebGPURenderer } from "three/webgpu";
export type RendererState = Awaited<ReturnType<typeof init>>;
export type ResolutionChangeHook = (state: RendererState) => void;

export async function init(canvas: HTMLCanvasElement, onResolutionChange: ResolutionChangeHook) {
	//tweak some default three.js settings for better performance
	Object3D.DEFAULT_MATRIX_AUTO_UPDATE = false;
	Object3D.DEFAULT_MATRIX_WORLD_AUTO_UPDATE = false;

	const state = {
		renderer: new WebGPURenderer({
			canvas,
			powerPreference: "high-performance",
			antialias: true,
		}),
		onResolutionChange,
		camera3D: new PerspectiveCamera(67),
		scene3D: new Scene(),
		camera2D: new OrthographicCamera(0, 1, 1, 0, -1, 1),
		scene2D: new Scene(),
	};

	await state.renderer.init();

	onresize(state);
	window.onresize = () => onresize(state);

	return state;
}

export function render(state: RendererState) {
	//3d
	state.renderer.autoClearColor = true;
	state.renderer.sortObjects = true;
	state.renderer.render(state.scene3D, state.camera3D);

	//2d
	state.renderer.autoClearColor = false;
	state.renderer.sortObjects = false;
	state.renderer.render(state.scene2D, state.camera2D);
}

function onresize(state: RendererState) {
	state.renderer.setSize(innerWidth, innerHeight);

	state.camera3D.aspect = innerWidth / innerHeight;
	state.camera3D.updateProjectionMatrix();

	state.camera2D.right = state.renderer.domElement.width;
	state.camera2D.top = state.renderer.domElement.height;
	state.camera2D.updateProjectionMatrix();

	state.onResolutionChange(state);
}
