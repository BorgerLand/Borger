import { BLUE, BOLD, BROWN, styledLog } from "@engine/client_ts/console_log.ts";

type CompatResult = {
	supported: boolean;
	hasFallback: boolean;
};

export async function testCompat() {
	const results = {
		"WASM SIMD": testSIMD(),
		"WebGL 2": testWebGL2(),
		WebGPU: await testWebGPU(),
		WebTransport: testWebTransport(),
		SharedArrayBuffer: testSharedArrayBuffer(),
		Touchscreen: testTouchscreen(),
	};
	const missing = [];

	for (const [test, result] of Object.entries(results)) {
		styledLog(false, [test, [BROWN, BOLD]], [": ", [BROWN]], [result.supported, [BLUE]]);
		if (!result.supported && !result.hasFallback) missing.push(test);
	}

	if (missing.length > 0) {
		const msg = `Your combination of browser/OS/device is missing these features required to play the game: ${missing.join(", ")}`;
		alert(msg);
		throw Error(msg);
	}

	return results;
}

function testSIMD(): CompatResult {
	return {
		supported: WebAssembly.validate(
			new Uint8Array([
				0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253,
				15, 253, 98, 11,
			]),
		),
		hasFallback: false,
	};
}

function testWebGL2(): CompatResult {
	const hasFallback = false;
	const gl = document.createElement("canvas").getContext("webgl2", { powerPreference: "high-performance" });
	if (!gl) return { supported: false, hasFallback };

	const gpuInfo = gl.getExtension("WEBGL_debug_renderer_info");
	if (gpuInfo)
		styledLog(
			false,
			["Selected GPU (WebGL 2)", [BROWN, BOLD]],
			[": ", [BROWN]],
			[gl.getParameter(gpuInfo.UNMASKED_RENDERER_WEBGL), [BLUE]],
		);

	return { supported: true, hasFallback };
}

async function testWebGPU(): Promise<CompatResult> {
	const hasFallback = true;
	const adapter = await navigator.gpu.requestAdapter();
	if (!adapter) return { supported: false, hasFallback };

	const info = adapter.info;
	styledLog(
		false,
		["Selected GPU (WebGPU)", [BROWN, BOLD]],
		[": ", [BROWN]],
		[`Vendor: ${info.vendor}, Architecture: ${info.architecture}`, [BLUE]],
	);

	return { supported: true, hasFallback };
}

function testSharedArrayBuffer(): CompatResult {
	return { supported: "SharedArrayBuffer" in window, hasFallback: false };
}

function testWebTransport(): CompatResult {
	//https://developer.apple.com/documentation/safari-release-notes/safari-26_4-release-notes#Networking
	//https://issues.chromium.org/issues/473215415
	const isChromiumAndroid = /Chrome/.test(navigator.userAgent) && /Android/.test(navigator.userAgent);

	return { supported: "WebTransport" in window && !isChromiumAndroid, hasFallback: true };
}

function testTouchscreen(): CompatResult {
	return { supported: !matchMedia("(hover: hover) and (pointer: fine)").matches, hasFallback: true };
}
