import js from "@eslint/js";
import globals from "globals";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsparser from "@typescript-eslint/parser";
import react from "eslint-plugin-react";
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
				project: true, //use tsconfig.json
			},
			sourceType: "module",
			globals: {
				...globals.es2021,
				...globals.browser,
				...globals.node,
				Bun: "readonly",
			},
		},
		plugins: {
			"@typescript-eslint": tseslint,
			react,
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
			...react.configs.recommended.rules,
			...react.configs["jsx-runtime"].rules,
			...reactHooks.configs.recommended.rules,

			"no-async-promise-executor": "off",
			"no-empty": "off",
			"no-mixed-spaces-and-tabs": "off",
			"no-inner-declarations": "off",
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
			"@typescript-eslint/no-this-alias": "off",
			"@typescript-eslint/no-explicit-any": "off",
			"@typescript-eslint/consistent-type-imports": "error",
			"@typescript-eslint/no-unused-vars": [
				"error",
				{
					argsIgnorePattern: "^_",
					varsIgnorePattern: "^_",
				},
			],

			//react-specific
			"react/prop-types": "off",
		},
	},
	...ciConfig,
] satisfies FlatConfig.ConfigArray;
