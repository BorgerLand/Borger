import { BLUE, BOLD, BROWN, styledLog } from "@engine/client_ts/console_log.ts";

export async function testCompat() {
	//logging is temporary for easy diagnostic purposes.
	//luckly each of these tests has a workaround if browser
	//doesn't support the requested feature
	logResult("SIMD", testSIMD());
	logResult("WebGPU", await testWebGPU());
	logResult("WebTransport", testWebTransport());
}

function logResult(test: string, result: boolean) {
	styledLog(false, [test, [BROWN, BOLD]], [": ", [BROWN]], [result, [BLUE]]);
}

function testSIMD() {
	return WebAssembly.validate(
		new Uint8Array([
			0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15,
			253, 98, 11,
		]),
	);
}

async function testWebGPU() {
	return (await navigator.gpu.requestAdapter()) !== null;
}

function testWebTransport() {
	return "WebTransport" in window;
}
