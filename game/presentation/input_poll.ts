//THIS SHOULD BE A SEPARATE NPM PACKAGE

import { Vector2, Vector3 } from "three";

//https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values#modifier_keys
export const MODIFIER_KEYS = Object.freeze([
	"alt",
	"altgraph",
	"capslock",
	"control",
	"fn",
	"fnlock",
	"hyper",
	"meta",
	"numlock",
	"scrolllock",
	"shift",
	"super",
	"symbol",
	"symbollock",
]);

//mouse button to pointer ID conversion enum
export enum MouseButton {
	LEFT,
	MIDDLE,
	RIGHT,
	BACK,
	FORWARD,
}

//key+pointer state enum
const ButtonState = {
	UP: undefined,
	DOWN: 0,
	DOWN_QUICK: 1, //pressed and released within the same frame
	JUST_PRESSED: 2,
} as const;

type ButtonStateValue = (typeof ButtonState)[keyof typeof ButtonState];

export class InputPoll {
	pixelRatio = devicePixelRatio;
	pointerPos = new Map<number, Vector2>();
	pointerDelta = new Map<number, Vector2>();
	scrollDelta = new Vector3();

	#keyState = new Map<string, ButtonStateValue>(); //keysAreDown(), keysAreJustPressed()
	#pointerState = new Map<number, ButtonStateValue>(); //isPointerDown(), isPointerJustPressed()

	#allowPointerLock = false; //setAllowPointerLock()
	#pointerLock = false; //isPointerLocked()

	#abortController: AbortController;
	#pointerLockAbort: AbortController | null = null;

	constructor(
		public canvas: HTMLCanvasElement,
		public touchscreenMode: boolean,
	) {
		if (!this.touchscreenMode) {
			//with mouse as hid, the mouse is the only pointer
			this.pointerPos.set(0, new Vector2());
			this.pointerDelta.set(0, new Vector2());
		}

		this.#abortController = new AbortController();
		const signal = this.#abortController.signal;

		document.addEventListener(
			"keydown",
			(e) => {
				if (!e.key || e.repeat) return;

				const key = e.key.toLowerCase();

				//prevent default tab behaviour when pointer lock is enabled
				//allow f# shortcuts (reload, devtools)
				if ((this.#pointerLock && (key === "tab" || !/^f\d+$/.test(key))) || key === "f11") {
					e.preventDefault();
				}

				this.#keyState.set(key, ButtonState.JUST_PRESSED);
			},
			{ signal },
		);

		document.addEventListener(
			"keyup",
			(e) => {
				if (!e.key) return;

				const key = e.key.toLowerCase();
				if (this.#keyState.get(key) === ButtonState.JUST_PRESSED) {
					this.#keyState.set(key, ButtonState.DOWN_QUICK);
				} else {
					this.#keyState.delete(key);
				}
			},
			{ signal },
		);

		document.addEventListener(
			"wheel",
			(e) => {
				const r = this.pixelRatio;
				this.scrollDelta.x += e.deltaX * r;
				this.scrollDelta.y += e.deltaY * r;
				this.scrollDelta.z += e.deltaZ * r;
			},
			{ signal },
		);

		canvas.addEventListener("contextmenu", (e) => e.preventDefault(), { signal });

		window.addEventListener("focus", () => this.reset(), { signal });
		window.addEventListener("blur", () => this.reset(), { signal });

		if (this.touchscreenMode) {
			canvas.addEventListener(
				"touchstart",
				(e) => {
					e.preventDefault();

					for (const touch of e.changedTouches) {
						const id = touch.identifier;
						const r = this.pixelRatio;

						this.#pointerState.set(id, ButtonState.JUST_PRESSED);
						this.pointerPos.set(
							id,
							new Vector2(touch.clientX * r, canvas.height - touch.clientY * r),
						);
						this.pointerDelta.set(id, new Vector2(0, 0));
					}
				},
				{ signal },
			);

			const handleTouchEnd = (e: TouchEvent) => {
				for (const touch of e.changedTouches) {
					const id = touch.identifier;
					if (this.#pointerState.get(id) === ButtonState.JUST_PRESSED) {
						this.#pointerState.set(id, ButtonState.DOWN_QUICK);
					} else {
						this.#pointerState.delete(id);
						this.pointerPos.delete(id);
						this.pointerDelta.delete(id);
					}
				}
			};

			canvas.addEventListener("touchend", handleTouchEnd, { signal });
			canvas.addEventListener("touchcancel", handleTouchEnd, { signal });

			canvas.addEventListener(
				"touchmove",
				(e) => {
					e.preventDefault();

					for (const touch of e.changedTouches) {
						const id = touch.identifier;
						const r = this.pixelRatio;
						const pointerDelta = this.pointerDelta.get(id)!;
						const pointerPos = this.pointerPos.get(id)!;

						const x = touch.clientX * r;
						const y = canvas.height - touch.clientY * r;
						pointerDelta.x += x - pointerPos.x;
						pointerDelta.y += y - pointerPos.y;
						pointerPos.set(x, y);
					}
				},
				{ signal },
			);
		} else {
			canvas.addEventListener(
				"mousedown",
				(e) => {
					this.#pointerState.set(e.button, ButtonState.JUST_PRESSED);
					if (!this.#pointerLock) canvas.requestPointerLock();
				},
				{ signal },
			);

			canvas.addEventListener(
				"mouseup",
				(e) => {
					const id = e.button;
					if (this.#pointerState.get(id) === ButtonState.JUST_PRESSED) {
						this.#pointerState.set(id, ButtonState.DOWN_QUICK);
					} else {
						this.#pointerState.delete(id);
					}
				},
				{ signal },
			);

			canvas.addEventListener(
				"mousemove",
				(e) => {
					const r = this.pixelRatio;

					this.pointerPos.get(0)!.set(e.clientX * r, canvas.height - e.clientY * r);
					const pointerDelta = this.pointerDelta.get(0)!;
					pointerDelta.x += e.movementX * r;
					pointerDelta.y -= e.movementY * r;
				},
				{ signal },
			);
		}
	}

	/**
	 * Check if a key combo is currently being pressed
	 * Usage: if(input.keysAreDown(["ctrl", "a"]))
	 * List of all checkable keys (remember to lower case-ify the string):
	 * https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values
	 */
	keysAreDown(keys: string[]) {
		//assert all keys in combo are pressed
		for (const k of keys) if (this.#keyState.get(k) === ButtonState.UP) return false;

		return this.#testCombo(keys);
	}

	/**
	 * Check if a key combo was just pressed
	 * Usage: if(input.keysAreJustPressed(["ctrl", "a"]))
	 * List of all checkable keys (remember to lower case-ify the string):
	 * https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values
	 */
	keysAreJustPressed(keys: string[]) {
		//assert all keys in combo are pressed
		for (let i = 0; i < keys.length; i++) {
			const state = this.#keyState.get(keys[i]);
			const last = i === keys.length - 1;

			if ((!last && state === ButtonState.UP) || (last && !state)) return false;
		}

		return this.#testCombo(keys);
	}

	/**
	 * Prompts the user to bind a new key combo.
	 * The promise resolves with a string array that can be used in keysAreDown or keysAreJustPressed.
	 * If escape is pushed, the promise is rejected
	 * @param {HTMLElement} keyDOM (optional) Element to display the current keys being entered.
	 */
	async promptNewKeyCombo(keyDOM?: HTMLElement) {
		return await new Promise<string[]>((resolve, reject) => {
			let combo: string[] = [];
			const abort = new AbortController();
			const signal = abort.signal;

			document.addEventListener(
				"keydown",
				(e) => {
					e.preventDefault();

					if (this.isPointerLocked()) return;

					const newKey = e.key.toLowerCase();
					if (newKey === "escape") {
						resolvePromise();
						return;
					}

					if (!combo.includes(newKey)) {
						combo.push(newKey);
						if (keyDOM) keyDOM.innerText = InputPoll.getKeyComboString(combo);
					}
				},
				{ signal },
			);

			document.addEventListener(
				"keyup",
				(e) => {
					e.preventDefault();

					if (this.isPointerLocked() || combo.length === 0) return;

					resolvePromise(combo);
				},
				{ signal },
			);

			document.addEventListener(
				"pointerlockchange",
				() => {
					combo = [];
				},
				{ signal },
			);

			function resolvePromise(result?: string[]) {
				abort.abort();
				if (result) resolve(result);
				else reject();
			}
		});
	}

	/**
	 * Transforms an array of key combinations to a human readable string.
	 * @param {string[]} keyCombos array of key combinations.
	 * @returns string
	 */
	static getKeyComboString(keyCombos: string[]) {
		return keyCombos
			.map((key) => (key === " " ? "space" : key))
			.map((key) => (key === "control" ? "ctrl" : key))
			.map((key) => key[0].toUpperCase() + key.substr(1)) //capitalize
			.join(" + ");
	}

	/**
	 * ID is either a MouseButton (desktop) or a pointer ID (mobile)
	 */
	isPointerDown(id: number) {
		return this.#pointerState.get(id) !== ButtonState.UP;
	}

	/**
	 * ID is either a MouseButton (desktop) or a pointer ID (mobile)
	 */
	isPointerJustPressed(id: number) {
		return Boolean(this.#pointerState.get(id));
	}

	/**
	 * Iterate over all pointer ID's who were just pressed since the last frame
	 */
	*getNewPointers() {
		for (const [id, state] of this.#pointerState.entries()) if (state) yield id;
	}

	isPointerLocked() {
		return this.#pointerLock;
	}

	getAllowPointerLock() {
		return this.#allowPointerLock;
	}

	setAllowPointerLock(allow: boolean) {
		if (this.touchscreenMode) {
			this.#allowPointerLock = allow;
			this.#pointerLock = allow;
		} else {
			if (allow && !this.#allowPointerLock) {
				this.#pointerLockAbort = new AbortController();
				document.addEventListener(
					"pointerlockchange",
					() => {
						this.reset();
						this.#pointerLock = document.pointerLockElement === this.canvas;
					},
					{ signal: this.#pointerLockAbort.signal },
				);
			} else if (!allow && this.#allowPointerLock) {
				this.#pointerLockAbort!.abort();
				this.#pointerLockAbort = null;

				this.#pointerLock = false;
				this.reset();
				document.exitPointerLock();
			}

			this.#allowPointerLock = allow;
		}
	}

	/**
	 * You must call this each animation frame, after polling is completed
	 */
	update() {
		for (const i of this.#keyState.keys()) {
			if (this.#keyState.get(i) === ButtonState.DOWN_QUICK) {
				this.#keyState.delete(i);
			} else {
				this.#keyState.set(i, ButtonState.DOWN);
			}
		}

		for (const i of this.#pointerState.keys()) {
			if (this.#pointerState.get(i) === ButtonState.DOWN_QUICK) {
				if (this.touchscreenMode) {
					this.pointerPos.delete(i);
					this.pointerDelta.delete(i);
				}

				this.#pointerState.delete(i);
			} else {
				this.#pointerState.set(i, ButtonState.DOWN);
			}
		}

		for (const pointerDelta of this.pointerDelta.values()) {
			pointerDelta.set(0, 0);
		}

		this.scrollDelta.set(0, 0, 0);
	}

	/**
	 * Removes all event listeners
	 */
	dispose() {
		this.setAllowPointerLock(false);
		this.#abortController.abort();
	}

	reset() {
		this.#keyState.clear();
		this.#pointerState.clear();

		if (this.touchscreenMode) {
			this.pointerPos.clear();
			this.pointerDelta.clear();
		} else {
			this.pointerDelta.get(0)!.set(0, 0);
		}

		this.scrollDelta.set(0, 0, 0);
	}

	#testCombo(keys: string[]) {
		if (keys.length > 1) {
			let comboPrvI;
			for (const i in this.#keyState) {
				//fail if modifier key is pressed that isn't in the combo (ctrl+c has no shift)
				if (InputPoll.#comboHasModifier(keys) && MODIFIER_KEYS.includes(i))
					for (const m of MODIFIER_KEYS) if (i === m && !keys.includes(m)) return false;

				//assert keys are pressed in the correct order (not c+ctrl)
				const comboCurI = keys.indexOf(i);
				if (comboCurI < 0) continue;
				if (comboCurI < comboPrvI!) return false;
				else comboPrvI = comboCurI;
			}
		}

		return true;
	}

	static #comboHasModifier(keys: string[]) {
		for (const k of keys) if (MODIFIER_KEYS.includes(k)) return true;

		return false;
	}
}
