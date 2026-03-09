import js from "@eslint/js";
import globals from "globals";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsparser from "@typescript-eslint/parser";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import prettier from "eslint-plugin-prettier/recommended";
import { readGitignoreFiles } from "eslint-gitignore";
import { globalIgnores } from "eslint/config";
import type { FlatConfig } from "@typescript-eslint/utils/ts-eslint";

const ciConfig = process.env.CI ? [prettier] : [];

export default [
	globalIgnores(
		readGitignoreFiles({ cwd: process.cwd() }).map((pattern) =>
			//dunno what's going on here
			pattern.startsWith("/") ? pattern.slice(1) : pattern,
		),
	),
	js.configs.recommended,
	{
		files: ["**/*.{js,ts,tsx}"],
		ignores: ["*.json"],
		languageOptions: {
			parser: tsparser,
			parserOptions: {
				project: true,
				tsconfigRootDir: import.meta.dirname,
			},
			sourceType: "module",
			globals: {
				...globals.es2026,
				...globals.browser,
				...globals.node,
				Bun: "readonly",
			},
		},
		plugins: {
			"@typescript-eslint": tseslint,
			"react-hooks": reactHooks,
			"react-refresh": reactRefresh,
		},
		settings: {
			react: {
				version: "detect",
			},
		},
		rules: {
			...tseslint.configs.recommended.rules,
			"no-async-promise-executor": "off",
			"no-empty": "off",
			"no-mixed-spaces-and-tabs": "off",
			"no-inner-declarations": "off",
			"preserve-caught-error": "off",
			"no-console": "error",
			eqeqeq: "error",
			"no-var": "error",
			"no-mixed-operators": [
				"error",
				{
					groups: [
						["&", "|", "^", "~", "<<", ">>", ">>>"],
						["==", "!=", "===", "!==", ">", ">=", "<", "<="],
						["&&", "||"],
						["in", "instanceof"],
					],
					allowSamePrecedence: true,
				},
			],
			"prefer-const": [
				"error",
				{
					destructuring: "all",
				},
			],
			"require-await": "error",
			"no-nested-ternary": "error",

			//ts-specific
			"no-undef": "off", //tsc handles this better than eslint
			"@typescript-eslint/no-this-alias": "off",
			"@typescript-eslint/no-explicit-any": "off",
			"@typescript-eslint/consistent-type-imports": "error",
			"@typescript-eslint/no-unused-vars": [
				"error",
				{
					argsIgnorePattern: "^_",
					varsIgnorePattern: "^_",
					caughtErrorsIgnorePattern: "^_",
				},
			],

			//react-specific
			...reactHooks.configs.recommended.rules,
			"react/prop-types": "off",
		},
	},
	...ciConfig,
] satisfies FlatConfig.ConfigArray;
