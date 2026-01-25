export function HUD() {
	return (
		<div className="pointer-events-none absolute left-0 top-0 text-xl text-white">
			Click the game to play, push escape to unlock the cursor
			<br />
			WASD - Movement
			<br />
			Space - Jump
			<br />
			Left click - Enable gravity
			<br />
			Right click - Blow nose
		</div>
	);
}
