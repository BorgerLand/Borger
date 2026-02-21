import { useTouchscreenStore } from "@game/simulation/input.ts";
import { Nipples } from "@game/presentation/ui/nipples.tsx";
import { Buttons } from "@game/presentation/ui/buttons.tsx";

export function HUD() {
	const touchscreenMode = useTouchscreenStore((s) => s.touchscreenMode);

	return (
		<div
			className="pointer-events-none absolute left-0 top-0 h-full w-full text-xl text-white"
			onContextMenu={(e) => e.preventDefault()}
		>
			{touchscreenMode ? (
				<>
					Left half of the screen - Move
					<br />
					Right half of the screen - Look
					<Nipples areolaSize={3.5} />
					<Buttons size={5} padding={0.75} />
				</>
			) : (
				<>
					Click the game to play, push escape to unlock the cursor
					<br />
					WASD - Movement
					<br />
					Space - Up
					<br />
					Shift - Down
				</>
			)}
		</div>
	);
}
