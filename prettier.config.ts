import { type Config } from "prettier";

export default {
	printWidth: 110,
	useTabs: true,
	tabWidth: 4,
	plugins: ["prettier-plugin-tailwindcss"],
	tailwindConfig: "tailwind.config.ts",
} satisfies Config;
