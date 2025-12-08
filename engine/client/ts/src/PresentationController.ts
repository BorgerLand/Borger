import * as Renderer from "@engine/client_ts/Renderer.ts";
import * as Net from "@engine/client_ts/Net.ts";
import ClientRSInit, * as ClientRS from "@engine/client_rs";
import * as ConsoleLog from "@engine/client_ts/ConsoleLog.ts";
import { BufferAttribute, InstancedBufferAttribute, Object3D, type TypedArray } from "three";
export type EngineState = Awaited<ReturnType<typeof init>>;

export async function init(cb: {
	canvasPromise: HTMLCanvasElement | Promise<HTMLCanvasElement>;
	onPresentationTick?: (state: EngineState) => void;
	onSpawnEntity?: (type: ClientRS.EntityKind) => Object3D;
	onDisposeEntity?: (type: ClientRS.EntityKind, entity: Object3D) => true;
	onResolutionChange?: (state: Renderer.RendererState) => void;
}) {
	//init procedure has been parallelized as much as possible
	const state = {
		dt: 0,
		prvTime: 0,

		...(await Promise.all([
			Net.init(),
			Promise.resolve(cb.canvasPromise).then(async function (canvas) {
				return await Renderer.init(canvas, cb.onResolutionChange ?? (() => {}));
			}),
			ClientRSInit(),
		]).then(function ([net, renderer]) {
			return new Promise<{
				renderer: Renderer.RendererState;
				rsInput: ClientRS.InputState;
				rsController: ClientRS.PresentationController;
			}>(function (resolve) {
				const rsController = new ClientRS.PresentationController(
					net.newClientSnapshot,
					net.inputStream,
					renderer.scene3D,
					cb?.onSpawnEntity ?? (() => new Object3D()),
					cb?.onDisposeEntity ?? (() => {}),
				);
				rsController.init_pinned(renderer.camera3D);

				const rsInput = new ClientRS.InputState();
				Net.onStateReceived(net, (buffer) => rsController.listen_for_state(buffer));

				//launch the simulation thread (by constructing ClientLocalEngineState),
				//then determine when it's ready by polling for a completed output.
				//it will tick twice before completing

				requestAnimationFrame(tryAgain);
				function tryAgain() {
					rsController.presentation_tick(0, rsInput);
					if (rsController.is_ready) resolve({ renderer, rsInput, rsController });
					else requestAnimationFrame(tryAgain);
				}
			});
		})),
	};

	ConsoleLog.init();

	state.renderer.renderer.setAnimationLoop(function (curTime: number) {
		//time keeping
		curTime /= 1000; //convert to seconds
		if (state.prvTime < 0) state.prvTime = curTime; //forces initial dt value to be 0
		state.dt = curTime - state.prvTime;
		state.prvTime = curTime;

		cb?.onPresentationTick?.(state);
	});

	return state;
}
