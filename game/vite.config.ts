import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import * as path from "path";
import packageJson from "../package.json";
import basicSsl from "@vitejs/plugin-basic-ssl";

//https://vite.dev/config/
export default defineConfig({
	publicDir: "assets",
	plugins: [react(), tailwindcss(), basicSsl()],
	resolve: {
		alias: {
			//should match tsconfig.json
			"@borger/rs": path.resolve("../engine/client/rs/pkg"),
			"@borger/ts": path.resolve("../engine/client/ts/src"),
			"@game": path.resolve("presentation"),
		},
	},
	server: {
		allowedHosts: true,
		headers: {
			//should match release.sh
			"Cross-Origin-Opener-Policy": "same-origin",
			"Cross-Origin-Embedder-Policy": "require-corp",
			"Cache-Control": "no-store, no-cache, must-revalidate",
			Pragma: "no-cache",
			Expires: "0",
			"Strict-Transport-Security": "max-age=31536000; includeSubDomains",
		},
	},
	optimizeDeps: {
		include: Object.entries(packageJson.dependencies)
			.map(([dep]) => dep)
			.concat("react-dom/client", "three/webgpu", "three/tsl", "three/examples/jsm/Addons.js"),
	},
	build: {
		outDir: "../release/client",
		rolldownOptions: {
			//can't bundle this because it's imported dynamically
			external: ["@borger/rs"],
		},
	},
});
