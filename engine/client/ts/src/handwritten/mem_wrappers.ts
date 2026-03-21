import * as ClientRS from "@borger/rs";

export let curLifetime = Number.MIN_SAFE_INTEGER;

export type State = ReturnType<typeof init>;

export function init() {
	return { offsets: ClientRS.get_mem_offsets(), memView: new DataView<ArrayBufferLike>(new ArrayBuffer()) };
}

export function invalidateBorrows(state: State, memory: ArrayBufferLike) {
	if (state.memView.buffer !== memory) state.memView = new DataView(memory);
	curLifetime++;
}

export function checkUseAfterFree(lifetime: number) {
	if (import.meta.env.DEV && lifetime !== curLifetime)
		throw ReferenceError("Use after free! This presentation state object is from a previous tick.");
}
