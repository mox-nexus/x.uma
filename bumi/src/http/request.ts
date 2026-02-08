/**
 * HTTP request context for matching.
 *
 * Query string is parsed from rawPath at construction. Headers are
 * stored lowercased for case-insensitive lookup.
 */
export class HttpRequest {
	readonly path: string;
	readonly queryParams: Readonly<Record<string, string>>;
	private readonly lowerHeaders: Record<string, string>;

	constructor(
		readonly method: string = "GET",
		readonly rawPath: string = "/",
		readonly headers: Readonly<Record<string, string>> = {},
	) {
		const qIdx = rawPath.indexOf("?");
		if (qIdx >= 0) {
			this.path = rawPath.slice(0, qIdx);
			const queryString = rawPath.slice(qIdx + 1);
			const params: Record<string, string> = {};
			for (const part of queryString.split("&")) {
				const eqIdx = part.indexOf("=");
				if (eqIdx >= 0) {
					params[part.slice(0, eqIdx)] = part.slice(eqIdx + 1);
				} else if (part) {
					params[part] = "";
				}
			}
			this.queryParams = params;
		} else {
			this.path = rawPath;
			this.queryParams = {};
		}

		this.lowerHeaders = {};
		for (const [k, v] of Object.entries(headers)) {
			this.lowerHeaders[k.toLowerCase()] = v;
		}
	}

	header(name: string): string | null {
		return this.lowerHeaders[name.toLowerCase()] ?? null;
	}

	queryParam(name: string): string | null {
		return this.queryParams[name] ?? null;
	}
}
