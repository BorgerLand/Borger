import { InputPoll } from "@engine/client_ts/InputPoll.ts";
import * as ClientRS from "@engine/client_rs";

let poll: InputPoll;
let rsInput: ClientRS.InputState;

const INPUT_SETTINGS = {
	sensitivity: 0.0025, //radians/pixel

	left: ["a"],
	right: ["d"],
	down: ["shift"],
	up: [" "],
	backward: ["s"],
	forward: ["w"],
	undo: ["control", "z"],
};

export function init(canvas: HTMLCanvasElement, rsInputObj: ClientRS.InputState) {
	poll = new InputPoll(canvas, false);
	poll.setAllowPointerLock(true);

	rsInput = rsInputObj;
}

export function update() {
	const pointerLocked = poll.isPointerLocked();
	if (pointerLocked) {
		const pointerDelta = poll.pointerDelta.get(0);
		const dx = pointerDelta?.x ?? 0;
		const dy = pointerDelta?.y ?? 0;

		const leftDown = poll.keysAreDown(INPUT_SETTINGS.left);
		const rightDown = poll.keysAreDown(INPUT_SETTINGS.right);
		const downDown = poll.keysAreDown(INPUT_SETTINGS.down);
		const upDown = poll.keysAreDown(INPUT_SETTINGS.up);
		const backwardDown = poll.keysAreDown(INPUT_SETTINGS.backward);
		const forwardDown = poll.keysAreDown(INPUT_SETTINGS.forward);

		ClientRS.populate_input(
			rsInput,
			dx * INPUT_SETTINGS.sensitivity,
			dy * INPUT_SETTINGS.sensitivity,
			Number(rightDown) - Number(leftDown),
			Number(upDown) - Number(downDown),
			Number(forwardDown) - Number(backwardDown),
		);
	} else {
		ClientRS.populate_input(rsInput, 0, 0, 0, 0, 0);
	}

	poll.update();
}

export function dispose() {
	poll.dispose();
}
