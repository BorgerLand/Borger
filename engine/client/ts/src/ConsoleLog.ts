export function styledLog(
	spaceBetweenSegments: boolean,
	...segments: [text: string | number | boolean, styles: string[]][]
) {
	const text = segments.map((segment) => `%c${segment[0]}`).join(spaceBetweenSegments ? " " : "");
	const styles = segments.map((segment) => segment[1].join(" "));
	styles.unshift(text);

	//eslint-disable-next-line no-console
	console.log.apply(console, styles);
}

export const BROWN = "color: #7f3a00;";
export const BLUE = "color: #0000c0;";
export const BOLD = "font-weight: bold;";
export const ITALIC = "font-style: italic;";
export const TITLE = [BOLD, ITALIC, "font-size: 57px;"];

export function init() {
	styledLog(false, [" 🍔", ["font-size: 140px;"]]);
	styledLog(false, [" 🅱️ORGER ", [BROWN, ...TITLE]]);

	styledLog(
		false,
		["B", [BROWN, BOLD]],
		["rowser-", [BROWN]],
		["O", [BROWN, BOLD]],
		["riented ", [BROWN]],
		["R", [BROWN, BOLD]],
		["ust ", [BROWN]],
		["G", [BROWN, BOLD]],
		["ame ", [BROWN]],
		["E", [BROWN, BOLD]],
		["ngine w/ ", [BROWN]],
		["R", [BROWN, BOLD]],
		["ancid tech stack", [BROWN]],
	);

	styledLog(
		true,
		["Load time:", [BROWN, BOLD]],
		[`${Math.round(performance.now() / 10) / 100} seconds`, [BROWN, ITALIC]],
	);
}
