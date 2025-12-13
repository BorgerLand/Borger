import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import * as path from "path";
import packageJson from "./package.json";

//https://vite.dev/config/
export default defineConfig({
	root: "game",
	publicDir: "assets",
	plugins: [react(), tailwindcss()],
	resolve: {
		alias: {
			//should match tsconfig.json
			"@engine/client_rs": path.resolve("engine/client/rs/pkg"),
			"@engine/client_ts": path.resolve("engine/client/ts/src"),
			"@simulation": path.resolve("game/ts/src/simulation"),
			"@presentation": path.resolve("game/ts/src/presentation"),
		},
	},
	server: {
		headers: {
			"Cross-Origin-Opener-Policy": "same-origin",
			"Cross-Origin-Embedder-Policy": "require-corp",
			"Cache-Control": "no-store",
		},
	},
	optimizeDeps: {
		include: Object.entries(packageJson.dependencies)
			.map(([dep]) => dep)
			.concat("react-dom/client", "three/webgpu", "three/tsl"),
	},
	build: {
		outDir: "../release/client",
		rolldownOptions: {
			//can't bundle this because the simulation thread
			//imports it in isolation
			external: ["@engine/client_rs"],
		},
	},
});
