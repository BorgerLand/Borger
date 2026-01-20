import { StrictMode, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { HUDHelp } from "@presentation/ui/HUDHelp.tsx";
import "@presentation/ui/index.css"; //what does it mean to import a css file. that makes no sense

export function init() {
	return new Promise<HTMLCanvasElement>(function (resolve) {
		createRoot(document.getElementById("root")!).render(
			<StrictMode>
				<div className="h-screen w-screen overflow-hidden">
					<GameCanvas />
					<HUDHelp />
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
