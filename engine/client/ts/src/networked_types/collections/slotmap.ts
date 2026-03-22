import * as MemWrappers from "@borger/ts/handwritten/mem_wrappers.ts";
import { SIZEOF_32BIT } from "@borger/ts/networked_types/primitive.ts";

export type SlotMap<T> = ((events?: {
	added: (id: number) => void;
	removed: (id: number) => void;
}) => Iterable<[number, T]> & { len: () => number; get: (id: number) => T | undefined }) & {
	len: () => number;
};

type SlotMapMemOffsets = {
	slot_0: number;
	slot_1: number;
	slots_stride: number;
	slots_ptr: number;
	slots_len: number;
	removed_ptr: number;
	removed_len: number;
	added_ptr: number;
	added_len: number;
};

export function wrap<T>(
	state: MemWrappers.State,
	ptr: number,
	offsets: SlotMapMemOffsets,
	wrapElement: (state: MemWrappers.State, ptr: number) => T,
	getElement: (ptr: number, id: number) => number | undefined,
): SlotMap<T> {
	const lifetime = state.curLifetime;
	const slotsPtr = state.memView.getUint32(ptr + offsets.slots_ptr, true);
	const slotsLen = state.memView.getUint32(ptr + offsets.slots_len, true);
	const removedPtr = state.memView.getUint32(ptr + offsets.removed_ptr, true);
	const removedLen = state.memView.getUint32(ptr + offsets.removed_len, true);
	const addedPtr = state.memView.getUint32(ptr + offsets.added_ptr, true);
	const addedLen = state.memView.getUint32(ptr + offsets.added_len, true);

	const len = () => slotsLen;
	const ret: SlotMap<T> = function (events) {
		if (events) {
			MemWrappers.checkUseAfterFree(state, lifetime);

			for (let i = 0; i < removedLen; i++)
				events.removed(state.memView.getUint32(removedPtr + i * SIZEOF_32BIT, true));
			for (let i = 0; i < addedLen; i++)
				events.added(state.memView.getUint32(addedPtr + i * SIZEOF_32BIT, true));
		}

		return {
			len,
			[Symbol.iterator]() {
				let i = 0;
				return {
					next() {
						if (i >= slotsLen) return { done: true, value: undefined };

						MemWrappers.checkUseAfterFree(state, lifetime);
						const slotPtr = slotsPtr + i++ * offsets.slots_stride;
						const id = state.memView.getUint32(slotPtr + offsets.slot_0, true);
						return { done: false, value: [id, wrapElement(state, slotPtr + offsets.slot_1)] };
					},
				};
			},

			get(id: number) {
				MemWrappers.checkUseAfterFree(state, lifetime);
				const elementPtr = getElement(ptr, id);
				if (elementPtr !== undefined) return wrapElement(state, elementPtr);
			},
		};
	};

	ret.len = len;
	return ret;
}
