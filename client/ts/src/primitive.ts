import * as MemWrappers from "@borger/ts/handwritten/mem_wrappers.ts";

export const SIZEOF_32BIT = 4; //32 bits / 8 = 4 bytes
export const SIZEOF_64BIT = 8; //64 bits / 8 = 8 bytes

export type Vec2 = ReturnType<typeof wrap_Vec2>;
export function wrap_Vec2(state: MemWrappers.State, ptr: number) {
	return {
		x: state.memView.getFloat32(ptr, true),
		y: state.memView.getFloat32(ptr + SIZEOF_32BIT, true),
	};
}

export function wrap_mut_Vec2(state: MemWrappers.State, ptr: number) {
	const lifetime = state.curLifetime;

	return {
		get_x() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr, true);
		},
		set_x(x: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr, x, true);
			return this;
		},
		get_y() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr + SIZEOF_32BIT, true);
		},
		set_y(y: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr + SIZEOF_32BIT, y, true);
			return this;
		},
	};
}

export type DVec2 = ReturnType<typeof wrap_DVec2>;
export function wrap_DVec2(state: MemWrappers.State, ptr: number) {
	return {
		x: state.memView.getFloat64(ptr, true),
		y: state.memView.getFloat64(ptr + SIZEOF_64BIT, true),
	};
}

export function wrap_mut_DVec2(state: MemWrappers.State, ptr: number) {
	const lifetime = state.curLifetime;

	return {
		get_x() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr, true);
		},
		set_x(x: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr, x, true);
			return this;
		},
		get_y() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr + SIZEOF_64BIT, true);
		},
		set_y(y: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr + SIZEOF_64BIT, y, true);
			return this;
		},
	};
}

export type Vec3 = ReturnType<typeof wrap_Vec3>;
export function wrap_Vec3(state: MemWrappers.State, ptr: number) {
	return {
		x: state.memView.getFloat32(ptr, true),
		y: state.memView.getFloat32(ptr + SIZEOF_32BIT, true),
		z: state.memView.getFloat32(ptr + 2 * SIZEOF_32BIT, true),
	};
}

export function wrap_mut_Vec3(state: MemWrappers.State, ptr: number) {
	const lifetime = state.curLifetime;

	return {
		get_x() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr, true);
		},
		set_x(x: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr, x, true);
			return this;
		},
		get_y() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr + SIZEOF_32BIT, true);
		},
		set_y(y: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr + SIZEOF_32BIT, y, true);
			return this;
		},
		get_z() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr + 2 * SIZEOF_32BIT, true);
		},
		set_z(z: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr + 2 * SIZEOF_32BIT, z, true);
			return this;
		},
	};
}

export type DVec3 = ReturnType<typeof wrap_DVec3>;
export function wrap_DVec3(state: MemWrappers.State, ptr: number) {
	return {
		x: state.memView.getFloat64(ptr, true),
		y: state.memView.getFloat64(ptr + SIZEOF_64BIT, true),
		z: state.memView.getFloat64(ptr + 2 * SIZEOF_64BIT, true),
	};
}

export function wrap_mut_DVec3(state: MemWrappers.State, ptr: number) {
	const lifetime = state.curLifetime;

	return {
		get_x() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr, true);
		},
		set_x(x: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr, x, true);
			return this;
		},
		get_y() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr + SIZEOF_64BIT, true);
		},
		set_y(y: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr + SIZEOF_64BIT, y, true);
			return this;
		},
		get_z() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr + 2 * SIZEOF_64BIT, true);
		},
		set_z(z: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr + 2 * SIZEOF_64BIT, z, true);
			return this;
		},
	};
}

export type Quat = ReturnType<typeof wrap_Quat>;
export function wrap_Quat(state: MemWrappers.State, ptr: number) {
	return {
		x: state.memView.getFloat32(ptr, true),
		y: state.memView.getFloat32(ptr + SIZEOF_32BIT, true),
		z: state.memView.getFloat32(ptr + 2 * SIZEOF_32BIT, true),
		w: state.memView.getFloat32(ptr + 3 * SIZEOF_32BIT, true),
	};
}

export function wrap_mut_Quat(state: MemWrappers.State, ptr: number) {
	const lifetime = state.curLifetime;

	return {
		get_x() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr, true);
		},
		set_x(x: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr, x, true);
			return this;
		},
		get_y() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr + SIZEOF_32BIT, true);
		},
		set_y(y: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr + SIZEOF_32BIT, y, true);
			return this;
		},
		get_z() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr + 2 * SIZEOF_32BIT, true);
		},
		set_z(z: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr + 2 * SIZEOF_32BIT, z, true);
			return this;
		},
		get_w() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat32(ptr + 3 * SIZEOF_32BIT, true);
		},
		set_w(w: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat32(ptr + 3 * SIZEOF_32BIT, w, true);
			return this;
		},
	};
}

export type DQuat = ReturnType<typeof wrap_DQuat>;
export function wrap_DQuat(state: MemWrappers.State, ptr: number) {
	return {
		x: state.memView.getFloat64(ptr, true),
		y: state.memView.getFloat64(ptr + SIZEOF_64BIT, true),
		z: state.memView.getFloat64(ptr + 2 * SIZEOF_64BIT, true),
		w: state.memView.getFloat64(ptr + 3 * SIZEOF_64BIT, true),
	};
}

export function wrap_mut_DQuat(state: MemWrappers.State, ptr: number) {
	const lifetime = state.curLifetime;

	return {
		get_x() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr, true);
		},
		set_x(x: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr, x, true);
			return this;
		},
		get_y() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr + SIZEOF_64BIT, true);
		},
		set_y(y: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr + SIZEOF_64BIT, y, true);
			return this;
		},
		get_z() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr + 2 * SIZEOF_64BIT, true);
		},
		set_z(z: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr + 2 * SIZEOF_64BIT, z, true);
			return this;
		},
		get_w() {
			MemWrappers.checkUseAfterFree(state, lifetime);
			return state.memView.getFloat64(ptr + 3 * SIZEOF_64BIT, true);
		},
		set_w(w: number) {
			MemWrappers.checkUseAfterFree(state, lifetime);
			state.memView.setFloat64(ptr + 3 * SIZEOF_64BIT, w, true);
			return this;
		},
	};
}
