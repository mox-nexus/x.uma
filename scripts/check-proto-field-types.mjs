#!/usr/bin/env node
/**
 * Keep `proto/xuma/**` inside the field types the config path round-trips exactly.
 *
 * A protojson config reaches the engine as JSON, is encoded to protobuf binary
 * to fill a `google.protobuf.Any`, and is decoded back to JSON for the input
 * factory. That round trip is lossless today, and the proof is short *because
 * the surface is small*: every field in every xuma message is a string, a bool,
 * bytes, a `map<string, string>`, or another xuma message. Under proto3 those
 * are the identity, modulo collapsing an explicit `""` to absent — which is
 * implicit presence, not a defect, and which every conforming protojson
 * implementation does the same way.
 *
 * Types outside that set are not identity:
 *
 *   int64 / uint64 / fixed64   protojson encodes them as JSON *strings*
 *   enum                       encoded as the value's *name*
 *   float / double             NaN and infinities become strings
 *   optional                   explicit presence, which the round trip erases
 *   Duration / Timestamp       string forms with their own grammar
 *   Struct / Value / Any       need a descriptor pool to expand
 *
 * None of those is unusable — they just make the round trip a transform rather
 * than an identity, and the day one appears the reasoning above has to be
 * redone. This check is what makes that day loud instead of silent. If you need
 * one of these types, delete the ban here and write the round-trip fixture that
 * replaces the proof.
 *
 * PLAN.md INV-FIELDTYPES.
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/\/$/, "");
const PROTO_DIR = join(ROOT, "proto", "xuma");

/** Scalar types whose protojson form is the identity of their JSON form. */
const ALLOWED_SCALARS = new Set(["string", "bool", "bytes"]);

function protoFiles(dir) {
	return readdirSync(dir).flatMap((entry) => {
		const path = join(dir, entry);
		return statSync(path).isDirectory()
			? protoFiles(path)
			: path.endsWith(".proto")
				? [path]
				: [];
	});
}

/** A field line: `  map<string, string> metadata = 2;` or `  string name = 1;` */
const FIELD = /^\s*(?:(repeated|optional)\s+)?([A-Za-z0-9_.<>, ]+?)\s+([a-z_][a-z0-9_]*)\s*=\s*\d+\s*;/;
const MAP = /^map<\s*([A-Za-z0-9_.]+)\s*,\s*([A-Za-z0-9_.]+)\s*>$/;

const problems = [];

for (const file of protoFiles(PROTO_DIR)) {
	const rel = file.slice(ROOT.length + 1);
	const lines = readFileSync(file, "utf8").split("\n");

	// Message names declared in this package are allowed as field types.
	const localMessages = new Set(
		lines.flatMap((l) => {
			const m = /^\s*message\s+([A-Za-z0-9_]+)\s*\{/.exec(l);
			return m ? [m[1]] : [];
		}),
	);

	lines.forEach((line, i) => {
		// `option` needs the trailing space: without it this also skips every
		// line starting `optional`, which is one of the things being banned.
		if (/^\s*(\/\/|option\s|import\s|package\s|syntax\s|message\s|enum\s|\}|$)/.test(line)) return;
		const m = FIELD.exec(line);
		if (!m) return;

		const [, modifier, rawType, name] = m;
		const type = rawType.trim();
		const at = `${rel}:${i + 1}`;

		if (modifier === "optional") {
			problems.push(`${at}  \`optional ${type} ${name}\` — explicit presence does not survive the round trip`);
			return;
		}

		const map = MAP.exec(type);
		if (map) {
			if (map[1] !== "string" || map[2] !== "string") {
				problems.push(`${at}  \`${type} ${name}\` — only map<string, string> round-trips as identity`);
			}
			return;
		}

		if (ALLOWED_SCALARS.has(type) || localMessages.has(type)) return;

		problems.push(`${at}  \`${type} ${name}\` — not a string, bool, bytes, map<string,string>, or a message in this file`);
	});
}

if (problems.length > 0) {
	console.error("check-proto-field-types: field types outside the round-trip-safe set\n");
	for (const p of problems) console.error(`  ${p}`);
	console.error(`
The config path encodes these to protobuf binary and decodes them back. The
types above do not survive that as the identity, so the reasoning recorded in
DECISIONS.md no longer holds. Either pick a listed type, or lift the ban here
and add a round-trip fixture that proves the new one behaves.`);
	process.exit(1);
}

console.log("check-proto-field-types: every xuma field round-trips as the identity");
