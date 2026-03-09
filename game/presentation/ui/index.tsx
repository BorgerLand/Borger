import { StrictMode, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { HUD } from "@game/presentation/ui/hud.tsx";
import "@game/presentation/ui/index.css"; //what does it mean to import a css file. that makes no sense

//warning this root component doesn't hot reload. changing it
//requires a full page refresh. try modifying hud instead
export function init() {
	return new Promise<HTMLCanvasElement>(function (resolve) {
		createRoot(document.getElementById("root")!).render(
			<StrictMode>
				<div
					className="h-screen w-screen touch-none select-none overflow-hidden overscroll-none"
					onContextMenu={(e) => e.preventDefault()}
				>
					<GameCanvas />
					<HUD />
				</div>
			</StrictMode>,
		);

		function GameCanvas() {
			const canvasRef = useRef<HTMLCanvasElement>(null);

			useEffect(() => {
				resolve(canvasRef.current as HTMLCanvasElement);
			}, []);

			return (
				<div className="h-screen w-screen overflow-hidden">
					<canvas ref={canvasRef} className="h-full w-full" />
				</div>
			);
		}
	});
}
