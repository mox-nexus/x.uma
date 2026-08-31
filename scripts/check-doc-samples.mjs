#!/usr/bin/env node
/**
 * Execute every code block on the getting-started pages, in the runtime that
 * owns it, and fail on any that does not run.
 *
 * # Why this exists
 *
 * `rumi-docs-tests` compiles the *Rust* blocks in `docs/content` via
 * `include_str!` + doctest, and has done since PR #26. Nothing ever did the
 * same for Python or TypeScript.
 *
 * The drift tracked that exactly. An audit on 2026-08-19 found that
 * `docs/content/getting-started/python.md` and
 * `rumi/crusts/python/README.md` both documented `load_http_matcher`, and
 * `typescript.md` documented `loadHttpMatcher` — a function that has never
 * existed in either crust. The two documents did not even agree with each
 * other: one passed a file path, the other passed the file's text. Everything
 * documenting the *gated* language was accurate; every defect was in the two
 * ungated ones.
 *
 * # Why only getting-started
 *
 * These pages are the curated path from nothing to a working match, so every
 * block on them has to run as written. Blocks elsewhere are legitimately
 * fragments — a predicate tree with its imports three fences up, an
 * illustrative shape that was never a whole program. Running those would
 * produce noise, and a gate that is mostly noise gets exclusions bolted on
 * until it asserts nothing. That failure mode is the one this repo keeps
 * finding, so the line is drawn where the claim is strongest instead.
 *
 * # Classes
 *
 * PLAN.md's M1 taxonomy, implemented. A block carries its class in an HTML
 * comment on the line before it; the default is `run`.
 *
 *     <!-- doc-sample: compile -->   type-checks, is not executed
 *     <!-- doc-sample: fragment -->  neither — not a whole program
 *
 * `compile` is for samples that are correct but do not terminate: the
 * `Bun.serve` example on the TypeScript page is a server, and demanding it
 * exit would be the gate being wrong rather than the doc.
 *
 * Both are visible, greppable admissions rather than silent skips. Marking a
 * broken block `fragment` to get to green is the failure this whole gate
 * exists to prevent, so the classes are deliberately few and deliberately
 * conspicuous.
 */

import { readFileSync, writeFileSync, mkdirSync, rmSync, existsSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");

/** A block on a getting-started page has no business taking 30 seconds. */
const TIMEOUT_MS = 30_000;

/** Which runtime owns a block, decided by what it imports. */
function ownerOf(lang, code) {
	if (lang === "python") {
		return /\bxuma_crust\b/.test(code) ? "crust-py" : "puma";
	}
	if (lang === "typescript") {
		return /["']xuma-crust["']/.test(code) ? "crust-wasm" : "bumi";
	}
	return null; // rust is covered by rumi-docs-tests; bash/yaml/toml are not code
}

/** Extract fenced blocks, carrying the opt-out marker that precedes them. */
function blocks(md) {
	const out = [];
	const re = /(?:<!--\s*doc-sample:\s*(\w+)\s*-->\s*\n)?```(\w+)\n([\s\S]*?)```/g;
	let m;
	let index = 0;
	while ((m = re.exec(md)) !== null) {
		out.push({ marker: m[1] ?? null, lang: m[2], code: m[3], index: index++ });
	}
	return out;
}

/**
 * Where a runtime's samples are written.
 *
 * Inside the owning package, not a temp dir: `bun run /tmp/x.ts` cannot resolve
 * `xuma`, because resolution walks up from the file and finds no node_modules.
 * A sample that fails on module resolution rather than on its own content is
 * exactly the noise that gets a gate disabled, so the sample is placed where a
 * reader's own file would be.
 */
const SANDBOX = {
	puma: "puma/.doc-samples",
	"crust-py": "rumi/crusts/python/.doc-samples",
	bumi: "bumi/.doc-samples",
	"crust-wasm": "rumi/crusts/wasm/.doc-samples",
};

const PAGES = [
	"docs/content/getting-started/python.md",
	"docs/content/getting-started/typescript.md",
	// The package READMEs are the *first* page a reader sees — npm and PyPI
	// render them on the landing page, before any doc site is reached. They were
	// ungated until 2026-08-23, and `bumi/README.md`'s HTTP example had three
	// defects: it called `new` on two type-only interfaces and passed an options
	// object to `HttpRequest`, whose constructor is positional. None of the three
	// survive being run; all three survived being read.
	"bumi/README.md",
	"puma/README.md",
	"rumi/crusts/python/README.md",
	"rumi/crusts/wasm/README.md",
	// Added 2026-08-23, after a `console.assert` repair on `concepts/pipeline.md`
	// left the block without its `assert` import and nothing noticed. The
	// original line — gate the getting-started pages only — was drawn to keep
	// the gate free of fragment noise. The noise turns out to be small and
	// nameable: seven blocks across these four pages are genuinely illustrative
	// and now say so in a marker, and everything else is a whole program that
	// has to run.
	"docs/content/concepts/pipeline.md",
	"docs/content/concepts/type-erasure.md",
	"docs/content/explain/architecture.md",
	"docs/content/how-to/share-config.md",
];

/** CI passes this: an environment-skipped block becomes a failure. */
const requireAll = process.argv.includes("--require-all");

const failures = [];
const ran = [];
const skipped = [];
const sandboxes = new Set();
const fixtures = new Set();

for (const page of PAGES) {
	const md = readFileSync(join(ROOT, page), "utf8");
	const all = blocks(md);

	// The page supplies its own config. Hand it to every block, so a sample
	// that reads routes.yaml is running against the file the reader would
	// have written from the same page.
	const yaml = all.find((b) => b.lang === "yaml");

	for (const b of all) {
		const owner = ownerOf(b.lang, b.code);
		if (owner === null) continue;
		if (b.marker === "fragment") {
			skipped.push(`${page} block ${b.index} (${b.lang}) — declared a fragment`);
			continue;
		}
		const cls = b.marker ?? "run";

		// The wasm crust resolves only from its wasm-pack output, which is not
		// present until it is built. Skipping is honest locally; in CI the
		// --require-all flag turns it into a failure, because "checked
		// somewhere else" is how the crusts broke three times in one day.
		if (owner === "crust-wasm" && !existsSync(join(ROOT, "rumi/crusts/wasm/pkg"))) {
			const why = `${page} block ${b.index} (crust-wasm) — pkg/ not built, run \`just crust-wasm-check\``;
			if (requireAll) failures.push(why);
			else skipped.push(why);
			continue;
		}

		const dir = join(ROOT, SANDBOX[owner]);
		mkdirSync(dir, { recursive: true });
		sandboxes.add(dir);

		// `routes.yaml` has to sit in the directory the sample actually runs
		// from, which differs by runtime: Python runs from the package root so
		// `uv` finds its project, TypeScript runs from the sandbox.
		const runDir = b.lang === "python" ? join(dir, "..") : dir;
		if (yaml) {
			const at = join(runDir, "routes.yaml");
			if (existsSync(at) && !fixtures.has(at)) {
				console.error(`check-doc-samples: refusing to overwrite ${at}`);
				process.exit(1);
			}
			writeFileSync(at, yaml.code);
			fixtures.add(at);
		}

		const name = b.lang === "python" ? `b${b.index}.py` : `b${b.index}.ts`;
		let code = b.code;
		if (owner === "crust-wasm") {
			// Published as `xuma-crust`; built locally to `pkg/`. Rewriting the
			// specifier is the one liberty this gate takes, and it is recorded
			// here rather than hidden: everything else about the sample —
			// including whether it initialises the module — is checked as written.
			code = code.replace(/["']xuma-crust["']/g, '"../pkg/xuma_crust.js"');
		}
		writeFileSync(join(dir, name), code);

		const res = cls === "compile" ? typecheck(owner, name, dir) : run(owner, name, dir);
		ran.push(`${page} block ${b.index} (${owner}, ${cls})`);
		if (res.status !== 0) {
			const detail = `${res.stderr || ""}${res.stdout || ""}`.trim().split("\n").slice(-4).join("\n");
			failures.push(`${page} block ${b.index} (${owner}, ${cls}) failed:\n${indent(detail)}`);
		}
	}
}

function indent(s) {
	return s
		.split("\n")
		.map((l) => `      ${l}`)
		.join("\n");
}

/**
 * Run one sample from inside its own package, as a reader's file would be.
 *
 * Bounded, because a sample that waits forever is a failure that hangs the
 * gate rather than reporting itself. A block on a getting-started page has no
 * business taking 30 seconds.
 */
/** Type-check without executing — for correct samples that never terminate. */
function typecheck(owner, name, cwd) {
	if (name.endsWith(".py")) {
		return spawnSync("uv", ["run", "mypy", "--ignore-missing-imports", name], {
			cwd,
			encoding: "utf8",
			timeout: TIMEOUT_MS,
		});
	}
	return spawnSync("bunx", ["tsc", "--noEmit", "--skipLibCheck", "--target", "esnext", "--module", "preserve", "--moduleResolution", "bundler", name], {
		cwd,
		encoding: "utf8",
		timeout: TIMEOUT_MS,
	});
}

function run(owner, name, cwd) {
	// `uv` discovers its project by walking up from the cwd, and a workspace
	// member's subdirectory resolves to the wrong root. Run from the package
	// root and reach down to the sample.
	const cmd = name.endsWith(".py")
		? ["uv", ["run", "python", join(".doc-samples", name)]]
		: ["bun", ["run", name]];
	const at = name.endsWith(".py") ? join(cwd, "..") : cwd;
	const res = spawnSync(cmd[0], cmd[1], { cwd: at, encoding: "utf8", timeout: TIMEOUT_MS });
	if (res.error?.code === "ETIMEDOUT" || res.signal === "SIGTERM") {
		return { status: 1, stdout: "", stderr: `did not finish within ${TIMEOUT_MS / 1000}s` };
	}
	return res;
}

for (const f of fixtures) rmSync(f, { force: true });
for (const dir of sandboxes) rmSync(dir, { recursive: true, force: true });

for (const s of skipped) console.log(`skip  ${s}`);
for (const r of ran) console.log(`run   ${r}`);

if (failures.length > 0) {
	console.error(`\ncheck-doc-samples: ${failures.length} block(s) do not run as written:\n`);
	for (const f of failures) console.error(`  ${f}\n`);
	console.error(
		"These are the pages a reader follows first. A block that does not run is\n" +
			"a promise the package does not keep. Fix the code or fix the doc — do not\n" +
			"mark it a fragment to make this pass.",
	);
	process.exit(1);
}

console.log(`\ncheck-doc-samples: ${ran.length} block(s) ran, ${skipped.length} declared fragments`);
