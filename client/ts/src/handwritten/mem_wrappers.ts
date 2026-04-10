import type { WASMBindgen } from "@borger/ts";

export type State = ReturnType<typeof init>;

export function init(wasmBindgen: WASMBindgen) {
	return {
		wasmBindgen,
		offsets: wasmBindgen.get_mem_offsets(),
		memView: new DataView<ArrayBufferLike>(new ArrayBuffer()),
		curLifetime: Number.MIN_SAFE_INTEGER,
	};
}

export function checkUseAfterFree(state: State, lifetime: number) {
	if (import.meta.env.DEV && lifetime !== state.curLifetime)
		throw ReferenceError("Use after free! This presentation state object is from a previous tick.");
}
