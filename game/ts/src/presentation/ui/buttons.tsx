import { useTouchscreenStore } from "@game/simulation/input.ts";

export function Buttons({ size, padding }: { size: number; padding: number }) {
	return (
		<div
			className="pointer-events-auto absolute flex flex-col"
			style={{ gap: `${padding}rem`, bottom: `${padding}rem`, right: `${padding}rem` }}
		>
			<Button text="▲" size={size} stateField="upButton" />
			<Button text="▼" size={size} stateField="downButton" />
		</div>
	);
}

function Button({
	text,
	size,
	stateField,
}: {
	text: string;
	size: number;
	stateField: "upButton" | "downButton";
}) {
	const pressed = useTouchscreenStore((s) => s[stateField]);

	return (
		<button
			className="flex touch-none items-center justify-center rounded-full"
			style={{
				width: `${size}rem`,
				height: `${size}rem`,
				background: pressed
					? "radial-gradient(circle, rgba(255,140,0,0.5) 0%, rgba(255,140,0,0.1) 100%)"
					: "radial-gradient(circle, rgba(128,128,128,0.6) 0%, rgba(128,128,128,0) 100%)",
				transition: "background 0.15s ease",
			}}
			onPointerDown={() => useTouchscreenStore.setState({ [stateField]: true })}
			onPointerUp={() => useTouchscreenStore.setState({ [stateField]: false })}
			onPointerLeave={() => useTouchscreenStore.setState({ [stateField]: false })}
		>
			<span className="text-2xl text-white">{text}</span>
		</button>
	);
}
