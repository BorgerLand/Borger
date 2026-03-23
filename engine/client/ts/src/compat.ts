import { BLUE, BOLD, BROWN, styledLog } from "@borger/ts/console_log.ts";
import type * as Borger from ".";

export type State = Awaited<ReturnType<typeof init>>;
export type Result = {
	name: string;
	supported: boolean;
	hasFallback: boolean;
};

export async function init(o?: Borger.PresentationInitOptions) {
	const results = {
		simd: testSIMD(),
		webGL2: testWebGL2(o?.requireWebGL2 ?? false),
		webGPU: await testWebGPU(o?.requireWebGPU ?? false),
		webTransport: testWebTransport(),
		sharedArrayBuffer: testSharedArrayBuffer(),
		touchscreen: testTouchscreen(),
	};
	const missing = [];

	for (const result of Object.values(results)) {
		styledLog(false, [result.name, [BROWN, BOLD]], [": ", [BROWN]], [result.supported, [BLUE]]);
		if (!result.supported && !result.hasFallback) missing.push(result.name);
	}

	if (missing.length > 0) {
		const msg = `Your combination of browser/OS/device is missing these features required to play the game: ${missing.join(", ")}`;
		alert(msg);
		throw Error(msg);
	}

	return results;
}

function testSIMD(): Result {
	return {
		name: "WASM SIMD",
		supported: WebAssembly.validate(
			new Uint8Array([
				0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253,
				15, 253, 98, 11,
			]),
		),
		hasFallback: false,
	};
}

function testWebGL2(required: boolean): Result {
	const name = "WebGL 2";
	const hasFallback = !required;
	const gl = document.createElement("canvas").getContext("webgl2", { powerPreference: "high-performance" });
	if (!gl) return { name, supported: false, hasFallback };

	const gpuInfo = gl.getExtension("WEBGL_debug_renderer_info");
	if (gpuInfo)
		styledLog(
			false,
			["Selected GPU (WebGL 2)", [BROWN, BOLD]],
			[": ", [BROWN]],
			[gl.getParameter(gpuInfo.UNMASKED_RENDERER_WEBGL), [BLUE]],
		);

	return { name, supported: true, hasFallback };
}

async function testWebGPU(required: boolean): Promise<Result> {
	const name = "WebGPU";
	const hasFallback = !required;
	const adapter = await navigator.gpu?.requestAdapter();
	if (!adapter) return { name, supported: false, hasFallback };

	const info = adapter.info;
	styledLog(
		false,
		["Selected GPU (WebGPU)", [BROWN, BOLD]],
		[": ", [BROWN]],
		[`Vendor: ${info.vendor}, Architecture: ${info.architecture}`, [BLUE]],
	);

	return { name, supported: true, hasFallback };
}

function testSharedArrayBuffer(): Result {
	return { name: "SharedArrayBuffer", supported: "SharedArrayBuffer" in window, hasFallback: false };
}

function testWebTransport(): Result {
	//https://developer.apple.com/documentation/safari-release-notes/safari-26_4-release-notes#Networking
	//https://issues.chromium.org/issues/473215415
	const isChromiumAndroid = /Chrome/.test(navigator.userAgent) && /Android/.test(navigator.userAgent);

	return {
		name: "WebTransport",
		supported: "WebTransport" in window && !isChromiumAndroid,
		hasFallback: true,
	};
}

function testTouchscreen(): Result {
	return {
		name: "Touchscreen",
		supported: !matchMedia("(hover: hover) and (pointer: fine)").matches,
		hasFallback: true,
	};
}
