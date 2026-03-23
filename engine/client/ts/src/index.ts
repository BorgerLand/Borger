import * as Net from "@borger/ts/net.ts";
import ClientRSInit, * as ClientRS from "@borger/rs";
import * as ConsoleLog from "@borger/ts/console_log.ts";
import * as Compat from "@borger/ts/compat.ts";
import * as MemWrappersH from "@borger/ts/handwritten/mem_wrappers.ts";
import * as MemWrappersG from "@borger/ts/generated/mem_wrappers.ts";

export type PresentationInitOptions = {
	hostname?: string; //eg. location.hostname, "localhost", "192.168.0.100", "server.com"
	webTransportPort?: number; //6969
	webSocketPort?: number; //6996

	requireWebGL2?: boolean;
	requireWebGPU?: boolean;
};

export type State = Awaited<ReturnType<typeof init>>;

export async function init(o?: PresentationInitOptions) {
	const compat = await Compat.init(o);

	//init the game server connection and wasm module in parallel
	const [wasm, net] = await Promise.all([
		ClientRSInit(),
		Net.init(
			o?.hostname ?? location.hostname,
			o?.webTransportPort ?? 6969,
			o?.webSocketPort ?? 6996,
			compat.webTransport.supported,
		),
	]);

	const wrappers = MemWrappersH.init();
	const rsController = new ClientRS.PresentationController(net.newClientSnapshot, net.writeInput);
	const rsInput = rsController.get_input_ptr();
	let initFrameRequest: number;

	Net.onStateReceived(net, (buffer) => rsController.listen_for_state(buffer));
	Net.onDisconnect(net, function () {
		rsController.abort_simulation();
		cancelAnimationFrame(initFrameRequest);
	});

	//launch the simulation thread (by constructing PresentationController)
	//and wait for it to be ready
	const { initTime, rsOutput } = await new Promise<{ initTime: number; rsOutput: number }>(function (
		resolve,
	) {
		initFrameRequest = requestAnimationFrame(tryAgain);
		function tryAgain(retryTime: number) {
			const rsOutput = rsController.presentation_tick(0);
			if (rsOutput !== undefined) {
				wrappers.memView = new DataView(wasm.memory.buffer);
				resolve({ initTime: retryTime, rsOutput });
			} else {
				initFrameRequest = requestAnimationFrame(tryAgain);
			}
		}
	});

	const state = {
		dt: 0,
		initTime,
		prvTime: -1,
		compat,
		wasm,
		net,
		rsController,
		rsInput,
		rsOutput,
		wrappers,
	};

	return state;
}

export function present(
	state: State,
	presentationLoop: (input: MemWrappersG.Input, output: MemWrappersG.Output) => void,
) {
	return new Promise<void>(function (resolve) {
		let frameRequest: number;
		Net.onDisconnect(state.net, function () {
			state.rsController.abort_simulation();
			cancelAnimationFrame(frameRequest);
			resolve();
		});

		animationFrame(state.initTime!);
		function animationFrame(curTime: number) {
			curTime /= 1000; //convert to seconds
			if (state.prvTime < 0) {
				//initial frame
				state.prvTime = curTime; //forces dt value to be 0
			} else {
				//ship the input off to the simulation + interpolate between simulation ticks
				state.rsOutput = state.rsController.presentation_tick(state.dt)!;

				const memory = state.wasm.memory.buffer;
				if (state.wrappers.memView.buffer !== memory) state.wrappers.memView = new DataView(memory);
			}

			state.dt = curTime - state.prvTime;
			state.prvTime = curTime;

			presentationLoop(
				MemWrappersG.wrap_Input(state.wrappers, state.rsInput),
				MemWrappersG.wrap_Output(state.wrappers, state.rsOutput),
			); //game on
			state.wrappers.curLifetime++;

			frameRequest = requestAnimationFrame(animationFrame);
		}

		ConsoleLog.init();
	});
}

export type * from "@borger/ts/networked_types/networked_types.ts";
export type * from "@borger/ts/generated/mem_wrappers.ts";
export { ClientDiscriminant } from "@borger/ts/generated/mem_wrappers.ts";
