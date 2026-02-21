import { useTouchscreenStore } from "@game/simulation/input.ts";
import type { Vector2 } from "three";

export function Nipples({ areolaSize }: { areolaSize: number }) {
	const move = useTouchscreenStore((s) => s.move);
	const look = useTouchscreenStore((s) => s.look);
	const dpr = useTouchscreenStore((s) => s.dpr);

	return (
		<div className="pointer-events-none absolute left-0 top-0 h-full w-full">
			{move.active && <Nipple start={move.start} dpr={dpr} areolaSize={areolaSize} />}
			{look.active && <Nipple start={look.start} dpr={dpr} areolaSize={areolaSize} />}
		</div>
	);
}

function Nipple({ start, dpr, areolaSize }: { start: Vector2; dpr: number; areolaSize: number }) {
	return (
		<div
			className="absolute -translate-x-1/2 -translate-y-1/2 rounded-full"
			style={{
				width: `${areolaSize}rem`,
				height: `${areolaSize}rem`,
				left: start.x / dpr,
				top: (window.innerHeight - start.y) / dpr,
				background: "radial-gradient(circle, rgba(128,128,128,0.6) 0%, rgba(128,128,128,0) 100%)",
			}}
		/>
	);
}
