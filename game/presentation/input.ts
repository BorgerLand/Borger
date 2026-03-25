import { InputPoll } from "@game/input_poll.ts";
import { Vector2 } from "three";
import { create } from "zustand";
import type * as Borger from "@borger/ts";

const tmpOmnidir = new Vector2();

export type TouchscreenStore = {
	touchscreenMode: boolean;
	dpr: number;
	move: NippleStore;
	look: NippleStore;

	upButton: boolean;
	downButton: boolean;
};

type NippleStore = { active: boolean; id: number; start: Vector2 };

export const useTouchscreenStore = create<TouchscreenStore>(() => ({
	touchscreenMode: false,
	dpr: devicePixelRatio,
	move: makeNipple(),
	look: makeNipple(),

	upButton: false,
	downButton: false,
}));

function makeNipple(): NippleStore {
	return { active: false, id: 0, start: new Vector2() };
}

export type State = ReturnType<typeof init>;

export function init(canvas: HTMLCanvasElement, touchscreenMode: boolean) {
	const poll = new InputPoll(canvas, touchscreenMode);
	poll.setAllowPointerLock(true);

	useTouchscreenStore.setState({ touchscreenMode, dpr: poll.pixelRatio });
	return {
		poll,
		yaw: 0,
		pitch: 0,
		settings: {
			lookSensitivityMouse: 0.0025, //radians/pixel
			lookSensitivityTouchscreen: 0.006, //radians/pixel
			moveSensitivityTouchscreen: 125, //pixels (drag distance required to hit full speed)

			left: ["a"],
			right: ["d"],
			down: ["shift"],
			up: [" "],
			backward: ["s"],
			forward: ["w"],
		},
	};
}

export function update(state: State, input: Borger.Input) {
	const pointerLocked = state.poll.isPointerLocked();
	if (pointerLocked) {
		if (state.poll.touchscreenMode) {
			let touchscreen = useTouchscreenStore.getState();

			if (touchscreen.move.active && !state.poll.isPointerDown(touchscreen.move.id))
				useTouchscreenStore.setState((s) => ({ move: { ...s.move, active: false } }));
			if (touchscreen.look.active && !state.poll.isPointerDown(touchscreen.look.id))
				useTouchscreenStore.setState((s) => ({ look: { ...s.look, active: false } }));

			touchscreen = useTouchscreenStore.getState();

			for (const newPointer of state.poll.getNewPointers()) {
				const pos = state.poll.pointerPos.get(newPointer)!;
				const halfWay = (state.poll.canvas.width * state.poll.pixelRatio) / 2;

				if (!touchscreen.move.active && pos.x < halfWay) {
					useTouchscreenStore.setState((s) => ({
						move: { ...s.move, active: true, id: newPointer, start: s.move.start.copy(pos) },
					}));
				} else if (!touchscreen.look.active && pos.x >= halfWay) {
					useTouchscreenStore.setState((s) => ({
						look: { ...s.look, active: true, id: newPointer, start: s.look.start.copy(pos) },
					}));
				}

				touchscreen = useTouchscreenStore.getState();
			}

			const movePos = touchscreen.move.active
				? state.poll.pointerPos.get(touchscreen.move.id)!
				: undefined;
			if (movePos) {
				tmpOmnidir.copy(movePos).sub(touchscreen.move.start);
				const len = Math.min(tmpOmnidir.length() / state.settings.moveSensitivityTouchscreen, 1);
				tmpOmnidir.setLength(len);
			} else {
				tmpOmnidir.set(0, 0);
			}

			const lookDelta = touchscreen.look.active
				? state.poll.pointerDelta.get(touchscreen.look.id)!
				: undefined;
			const lookDX = lookDelta?.x ?? 0;
			const lookDY = lookDelta?.y ?? 0;

			state.yaw -= lookDX * state.settings.lookSensitivityTouchscreen;
			state.pitch += lookDY * state.settings.lookSensitivityTouchscreen;

			input.omnidir
				.set_x(tmpOmnidir.x)
				.set_y(Number(touchscreen.upButton) - Number(touchscreen.downButton))
				.set_z(tmpOmnidir.y);
		} else {
			const pointerDelta = state.poll.pointerDelta.get(0);
			const lookDX = pointerDelta?.x ?? 0;
			const lookDY = pointerDelta?.y ?? 0;

			const leftKey = state.poll.keysAreDown(state.settings.left);
			const rightKey = state.poll.keysAreDown(state.settings.right);
			const backwardKey = state.poll.keysAreDown(state.settings.backward);
			const downKey = state.poll.keysAreDown(state.settings.down);
			const upKey = state.poll.keysAreDown(state.settings.up);
			const forwardKey = state.poll.keysAreDown(state.settings.forward);

			state.yaw -= lookDX * state.settings.lookSensitivityMouse;
			state.pitch += lookDY * state.settings.lookSensitivityMouse;

			input.omnidir
				.set_x(Number(rightKey) - Number(leftKey))
				.set_y(Number(upKey) - Number(downKey))
				.set_z(Number(forwardKey) - Number(backwardKey));
		}
	}

	input.set_cam_yaw(state.yaw).set_cam_pitch(state.pitch);

	//call input::validate() (the rust code) in order to reuse its clamping logic
	input.validate();
	state.yaw = input.get_cam_yaw();
	state.pitch = input.get_cam_pitch();

	state.poll.update();
}

export function dispose(state: State) {
	state.poll.dispose();
}
