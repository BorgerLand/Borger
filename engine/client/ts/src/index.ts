import * as Net from "@borger/ts/net.ts";
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

//eslint-disable-next-line @typescript-eslint/consistent-type-imports
export type WASMBindgen = typeof import("@borger/rs");

async function initWASM(singlethreaded: boolean) {
	let wasmBindgen: WASMBindgen;
	if (import.meta.env.DEV) wasmBindgen = await import("@borger/rs");
	else
		wasmBindgen = await import(/* @vite-ignore */ `/assets/client_rs_${singlethreaded ? "st" : "mt"}.js`);

	const wasmModule = await wasmBindgen.default();

	return { wasmBindgen, wasmModule };
}

export async function init(o?: PresentationInitOptions) {
	const compat = await Compat.init(o);

	//init the game server connection and wasm module in parallel
	const [{ wasmBindgen, wasmModule }, net] = await Promise.all([
		initWASM(!compat.sharedArrayBuffer.supported),
		Net.init(
			o?.hostname ?? location.hostname,
			o?.webTransportPort ?? 6969,
			o?.webSocketPort ?? 6996,
			compat.webTransport.supported,
		),
	]);

	const wrappers = MemWrappersH.init(wasmBindgen);
	const rsController = new wasmBindgen.PresentationController(net.newClientSnapshot, net.writeInput);
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
				wrappers.memView = new DataView(wasmModule.memory.buffer);
				resolve({ initTime: retryTime, rsOutput });
			} else {
				initFrameRequest = requestAnimationFrame(tryAgain);
			}
		}
	});

	if ((rsController as any).dump_session) {
		(globalThis as any).dumpSession = function () {
			const blob = new Blob([(rsController as any).dump_session() as any], {
				type: "application/octet-stream",
			});
			const url = URL.createObjectURL(blob);
			const a = document.createElement("a");
			a.href = url;
			a.download = "borger.dump";
			a.click();
			URL.revokeObjectURL(url);
		};
	}

	const state = {
		dt: 0,
		initTime,
		prvTime: -1,
		compat,
		wasmModule,
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

				const memory = state.wasmModule.memory.buffer;
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

export async function replaySession() {
	const wasmBindgen = (await initWASM(false)).wasmBindgen;

	//eslint-disable-next-line no-console
	console.log(
		"Drag a session dump file onto the page, or click anywhere on the page to open a file prompt.",
	);

	const file = await new Promise<Uint8Array>((resolve, reject) => {
		function cleanup() {
			document.removeEventListener("dragover", onDragOver);
			document.removeEventListener("drop", onDrop);
			document.removeEventListener("click", onClick);
		}

		function onDragOver(e: DragEvent) {
			e.preventDefault();
		}

		async function onDrop(e: DragEvent) {
			e.preventDefault();
			cleanup();

			const file = e.dataTransfer?.files?.[0];
			if (!file) return reject("No file dropped");

			const buffer = await file.arrayBuffer();
			resolve(new Uint8Array(buffer));
		}

		function onClick() {
			cleanup();

			const input = document.createElement("input");
			input.type = "file";
			input.accept = ".dump";

			input.onchange = async function (e) {
				const file = (e.target as HTMLInputElement).files?.[0];
				if (!file) return reject("No file selected");

				const buffer = await file.arrayBuffer();
				resolve(new Uint8Array(buffer));
			};

			input.click();
		}

		document.addEventListener("dragover", onDragOver);
		document.addEventListener("drop", onDrop);
		document.addEventListener("click", onClick);
	});

	(wasmBindgen.PresentationController as any).replay_session(file);
}

export type * from "@borger/ts/networked_types/networked_types.ts";
export type * from "@borger/ts/generated/mem_wrappers.ts";
export { ClientDiscriminant } from "@borger/ts/generated/mem_wrappers.ts";
