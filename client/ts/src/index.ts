import * as Net from "@borger/ts/net.ts";
import * as ConsoleLog from "@borger/ts/console_log.ts";
import * as Compat from "@borger/ts/compat.ts";
import * as MemWrappersH from "@borger/ts/handwritten/mem_wrappers.ts";
import * as MemWrappersG from "@borger/ts/generated/mem_wrappers.ts";
import type * as WASMBindgenNS from "@borger/rs";

export type PresentationInitOptions =
	| {
			hostname?: string; //eg. location.hostname, "localhost", "192.168.0.100", "server.com"
			webTransportPort?: number; //6969
			webSocketPort?: number; //6996
			onConnectionChange?: (oops?: { message: string; attempt: number }) => void | Promise<void>;

			requireWebGL2?: boolean;
			requireWebGPU?: boolean;
	  }
	| undefined;

export type WASMBindgen = typeof WASMBindgenNS;
type WASMState = Awaited<ReturnType<typeof initWASM>>;

async function initWASM(singlethreaded: boolean) {
	let bindgen: WASMBindgen;
	if (import.meta.env.DEV) bindgen = await import("@borger/rs");
	else bindgen = await import(/* @vite-ignore */ `/assets/client_rs_${singlethreaded ? "st" : "mt"}.js`);

	const module = await bindgen.default();

	return { bindgen, module };
}

type PresentationLoop = (dt: number, ctx: MemWrappersG.GameContext) => void;

export async function play(
	initGameCB: (compat: Compat.State) => PresentationLoop | Promise<PresentationLoop>,
	o?: PresentationInitOptions,
) {
	const compat = await Compat.init(o);
	const [wasm, presentationLoop] = await Promise.all([
		initWASM(!compat.sharedArrayBuffer.supported),
		initGameCB(compat),
	]);

	const initialSession = await initConnectionSession(o, wasm, compat);
	if (!initialSession) return;
	let connectionSession = initialSession;

	let dt = 0,
		prvTime = -1;

	await new Promise<void>(function (resolve) {
		if (connectionSession.userCancel instanceof Promise) connectionSession.userCancel.then(resolve);

		animationFrame(connectionSession.initTime);
		async function animationFrame(curTime: number) {
			let resumed = false;
			if (typeof connectionSession.net.oops === "string") {
				//connection failed during gameplay. try to reconnect under a new
				//connection session and then resume the presentation loop. this
				//is possible without a major reload because presentation will
				//just resume rendering whatever the new simulation tells it to
				//render, with the only side effect being a single large dt
				const newSession = await initConnectionSession(o, wasm, compat, connectionSession);
				if (!newSession) {
					//user cancelled/gave up
					resolve();
					return;
				}

				if (newSession.userCancel instanceof Promise) newSession.userCancel.then(resolve);

				connectionSession = newSession;
				curTime = connectionSession.initTime;
				resumed = true;
			}

			curTime /= 1000; //convert to seconds
			if (prvTime < 0) {
				prvTime = curTime; //forces initial dt value to be 0
			} else if (!resumed) {
				//ship the input off to the simulation + interpolate between simulation ticks
				connectionSession.rsOutput = connectionSession.rsController.presentation_tick(dt)!;
			}

			dt = curTime - prvTime;
			prvTime = curTime;

			const memory = wasm.module.memory.buffer;
			if (connectionSession.wrappers.memView.buffer !== memory)
				connectionSession.wrappers.memView = new DataView(memory);

			const input = MemWrappersG.wrap_Input(connectionSession.wrappers, connectionSession.rsInput);
			const ctx = MemWrappersG.wrap_GameContext(
				connectionSession.wrappers,
				connectionSession.rsOutput,
				input,
			);

			presentationLoop(dt, ctx); //game on
			connectionSession.wrappers.curLifetime++;

			requestAnimationFrame(animationFrame);
		}

		ConsoleLog.init();
	});
}

type ConnectionSession = {
	net: Net.State;
	rsController: WASMBindgenNS.PresentationController;
	rsInput: number;
	initTime: number;
	rsOutput: number;
	wrappers: MemWrappersH.State;
	userCancel: void | Promise<void>;
};

//represents the data whose lifetime spans from connection opening to disconnecting
async function initConnectionSession(
	o: PresentationInitOptions,
	wasm: WASMState,
	compat: Compat.State,
	resumeFrom?: ConnectionSession,
): Promise<ConnectionSession | undefined> {
	//awkward co-dependency between presentation controller and net
	const onStateReceivedPromise = Promise.withResolvers<(stateBuffer: Uint8Array) => void>();

	let net,
		userCancel,
		attempt = 1;

	userCancel = o?.onConnectionChange?.({
		message: resumeFrom ? resumeFrom.net.oops! : "Connecting...",
		attempt,
	});

	//keep retrying until success or cancel
	while (true) {
		const netResult = await Net.init(
			o?.hostname ?? location.hostname,
			o?.webTransportPort ?? 6969,
			o?.webSocketPort ?? 6996,
			compat.webTransport.supported,
			onStateReceivedPromise.promise,
			userCancel,
		);

		if (netResult.type === Net.NetResultType.Err) {
			userCancel = o?.onConnectionChange?.({ message: netResult.result, attempt: ++attempt });
			await new Promise((resolve) => setTimeout(resolve, 500));
		} else if (netResult.type === Net.NetResultType.UserCancel) {
			return;
		} else {
			net = netResult.result;
			break;
		}
	}

	let rsController: WASMBindgenNS.PresentationController;
	if (!resumeFrom)
		rsController = new wasm.bindgen.PresentationController(net.newClientSnapshot, net.writeInput);
	else
		rsController = wasm.bindgen.PresentationController.resume_disconnected_session(
			resumeFrom.rsController,
			net.newClientSnapshot,
			net.writeInput,
		);

	onStateReceivedPromise.resolve((buffer) => rsController.listen_for_state(buffer));
	const rsInput = rsController.get_input_ptr();
	const wrappers = MemWrappersH.init(wasm.bindgen);

	//launch the simulation thread (by constructing PresentationController)
	//and wait for it to be ready
	const { initTime, rsOutput } = await new Promise<{ initTime: number; rsOutput: number }>(function (
		resolve,
	) {
		requestAnimationFrame(tryAgain);
		function tryAgain(retryTime: number) {
			const rsOutput = rsController.presentation_tick(0);
			if (rsOutput !== undefined) {
				resolve({ initTime: retryTime, rsOutput });
			} else {
				requestAnimationFrame(tryAgain);
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

	userCancel = o?.onConnectionChange?.();

	return {
		net,
		rsController,
		rsInput,
		initTime,
		rsOutput,
		wrappers,
		userCancel,
	};
}

export async function replaySession() {
	const wasmBindgen = (await initWASM(false)).bindgen;

	ConsoleLog.styledLog(false, [
		"Drag a session dump file onto the page, or click anywhere on the page to open a file prompt.",
		[ConsoleLog.BROWN, ConsoleLog.BOLD],
	]);

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
