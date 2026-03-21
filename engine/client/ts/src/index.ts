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
	game: (state: Init) => Promise<(input: MemWrappersG.Input, output: MemWrappersG.Output) => void>; //an async init function that returns a loop function
};

export type Init = {
	dt: number;
	compat: Compat.State;
};

export function init(o: PresentationInitOptions) {
	return new Promise<void>(async function (resolve) {
		const compat = await Compat.init();

		//init the game server connection and wasm module in parallel
		const [wasm, net] = await Promise.all([
			ClientRSInit(),
			Net.init(
				o.hostname ?? location.hostname,
				o.webTransportPort ?? 6969,
				o.webSocketPort ?? 6996,
				compat.WebTransport.supported,
			),
		]);

		const output = MemWrappersH.init();
		const rsController = new ClientRS.PresentationController(net.newClientSnapshot, net.writeInput);
		const rsInput = rsController.get_input_ptr();
		let frameRequest: number;

		Net.onStateReceived(net, (buffer) => rsController.listen_for_state(buffer));
		Net.onDisconnect(net, function () {
			rsController.abort_simulation();
			cancelAnimationFrame(frameRequest);
			resolve();
		});

		//launch the simulation thread (by constructing PresentationController)
		//and wait for it to be ready
		let { initTime, rsOutput } = await new Promise<{ initTime: number; rsOutput: number }>(function (
			resolve,
		) {
			frameRequest = requestAnimationFrame(tryAgain);
			function tryAgain(retryTime: number) {
				const rsOutput = rsController.presentation_tick(0);
				if (rsOutput !== undefined) {
					MemWrappersH.invalidateBorrows(output, wasm.memory.buffer);
					resolve({ initTime: retryTime, rsOutput });
				} else {
					frameRequest = requestAnimationFrame(tryAgain);
				}
			}
		});

		let prvTime = -1;
		const state = {
			dt: 0,
			compat,
		};

		const cbLoop = await o.game(state);
		presentationLoop(initTime!);
		function presentationLoop(curTime: number) {
			curTime /= 1000; //convert to seconds
			if (prvTime < 0) {
				//initial frame
				prvTime = curTime; //forces dt value to be 0
			} else {
				//ship the input off to the simulation + interpolate between simulation ticks
				rsOutput = rsController.presentation_tick(state.dt)!;
			}

			state.dt = curTime - prvTime;
			prvTime = curTime;

			cbLoop(MemWrappersG.wrap_Input(output, rsInput), MemWrappersG.wrap_Output(output, rsOutput)); //game on
			MemWrappersH.invalidateBorrows(output, wasm.memory.buffer);

			frameRequest = requestAnimationFrame(presentationLoop);
		}

		ConsoleLog.init();
	});
}

export type * from "@borger/ts/networked_types/networked_types.ts";
export type * from "@borger/ts/generated/mem_wrappers.ts";
export { ClientDiscriminant } from "@borger/ts/generated/mem_wrappers.ts";
