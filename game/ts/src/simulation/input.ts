import { InputPoll } from "@engine/client_ts/input_poll.ts";
import * as ClientRS from "@engine/client_rs";
import { Vector2 } from "three";
import { create } from "zustand";

let poll: InputPoll;
let rsInput: ClientRS.InputState;

const tmpOmnidir = new Vector2();

const INPUT_SETTINGS = {
	lookSensitivityMouse: 0.0025, //radians/pixel
	lookSensitivityTouchscreen: 0.006, //radians/pixel
	moveSensitivityTouchscreen: 125, //pixels (drag distance required to hit full speed)

	left: ["a"],
	right: ["d"],
	down: ["shift"],
	up: [" "],
	backward: ["s"],
	forward: ["w"],
	undo: ["control", "z"],
};

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

export function init(canvas: HTMLCanvasElement, rsInputObj: ClientRS.InputState, touchscreenMode: boolean) {
	poll = new InputPoll(canvas, touchscreenMode);
	poll.setAllowPointerLock(true);

	rsInput = rsInputObj;

	useTouchscreenStore.setState({ touchscreenMode, dpr: poll.pixelRatio });
}

export function update() {
	const pointerLocked = poll.isPointerLocked();
	if (pointerLocked) {
		if (poll.touchscreenMode) {
			let touchscreen = useTouchscreenStore.getState();

			if (touchscreen.move.active && !poll.isPointerDown(touchscreen.move.id))
				useTouchscreenStore.setState((s) => ({ move: { ...s.move, active: false } }));
			if (touchscreen.look.active && !poll.isPointerDown(touchscreen.look.id))
				useTouchscreenStore.setState((s) => ({ look: { ...s.look, active: false } }));

			touchscreen = useTouchscreenStore.getState();

			for (const newPointer of poll.getNewPointers()) {
				const pos = poll.pointerPos.get(newPointer)!;
				const halfWay = (poll.canvas.width * poll.pixelRatio) / 2;

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

			const movePos = touchscreen.move.active ? poll.pointerPos.get(touchscreen.move.id)! : undefined;
			if (movePos) {
				tmpOmnidir.copy(movePos).sub(touchscreen.move.start);
				const len = Math.min(tmpOmnidir.length() / INPUT_SETTINGS.moveSensitivityTouchscreen, 1);
				tmpOmnidir.setLength(len);
			} else {
				tmpOmnidir.set(0, 0);
			}

			const lookDelta = touchscreen.look.active
				? poll.pointerDelta.get(touchscreen.look.id)!
				: undefined;
			const lookDX = lookDelta?.x ?? 0;
			const lookDY = lookDelta?.y ?? 0;

			ClientRS.populate_input(
				rsInput,
				lookDX * INPUT_SETTINGS.lookSensitivityTouchscreen,
				lookDY * INPUT_SETTINGS.lookSensitivityTouchscreen,
				tmpOmnidir.x,
				Number(touchscreen.upButton) - Number(touchscreen.downButton),
				tmpOmnidir.y,
			);
		} else {
			const pointerDelta = poll.pointerDelta.get(0);
			const lookDX = pointerDelta?.x ?? 0;
			const lookDY = pointerDelta?.y ?? 0;

			const leftKey = poll.keysAreDown(INPUT_SETTINGS.left);
			const rightKey = poll.keysAreDown(INPUT_SETTINGS.right);
			const backwardKey = poll.keysAreDown(INPUT_SETTINGS.backward);
			const downKey = poll.keysAreDown(INPUT_SETTINGS.down);
			const upKey = poll.keysAreDown(INPUT_SETTINGS.up);
			const forwardKey = poll.keysAreDown(INPUT_SETTINGS.forward);

			ClientRS.populate_input(
				rsInput,
				lookDX * INPUT_SETTINGS.lookSensitivityMouse,
				lookDY * INPUT_SETTINGS.lookSensitivityMouse,
				Number(rightKey) - Number(leftKey),
				Number(upKey) - Number(downKey),
				Number(forwardKey) - Number(backwardKey),
			);
		}
	} else {
		ClientRS.populate_input(rsInput, 0, 0, 0, 0, 0);
	}

	poll.update();
}

export function dispose() {
	poll.dispose();
}
