//! Does the fixture corpus span the schema?
//!
//! This is the price of not generating types for puma and bumi
//! (`DECISIONS.md` D-038). With generated types, the dependency on
//! `proto/xuma/**` is an arrow the build can see: change a field, the compile
//! breaks. Reading protojson by hand makes that arrow a human's memory. The
//! arrow has to come back some other way, and this is it — the fixture corpus
//! becomes the thing that says what the schema is, and the conformance
//! ledger propagates it to every implementation.
//!
//! Without this, "all three implementations agree" means only "all three agree
//! about whatever somebody remembered to fixture".
//!
//! # What is checked
//!
//! Every message in `proto/xuma/**` either appears as an `@type` in a fixture
//! with all of its fields set, or is named in [`KNOWN_GAPS`] with a reason. The
//! gap list is the visible form of the shortfall, and it is checked for
//! staleness too: an entry whose message is now covered, or no longer exists,
//! fails the test.
//!
//! # What is deliberately not checked
//!
//! That every field also has a *misspelling* fixture proving rejection.
//! `protojson_rejects_an_unknown_field` proves the mechanism once, and the
//! mechanism is schema-wide — every generated deserializer ends its field
//! visitor with `unknown_field`. Per-field versions would be fifteen copies of
//! one proof. If a hand-written reader in puma or bumi ever replaces that
//! mechanism with something per-field, this stops being true and the per-field
//! version becomes necessary.

#![cfg(all(feature = "fixtures", feature = "registry"))]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Messages with no protojson fixture, and why. This list should only shrink.
const KNOWN_GAPS: &[(&str, &str)] = &[
    (
        "xuma.claude.v1.HookContext",
        "context data, not configuration — it is what a matcher reads, never an @type payload",
    ),
    (
        "xuma.core.v1.BoolMatcher",
        "no shipped input produces a boolean, so a positive fixture is not expressible in the \
         kv domain this runner uses. The negative one — an incompatible input/matcher pair — \
         is rejected at load by rumi and accepted by puma, because `supported_types` \
         validation exists in Rust only. Fixture it when that divergence is closed.",
    ),
    (
        "xuma.claude.v1.AllowAction",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.BlockAction",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.ModifyAction",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.EventTypeInput",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.ToolNameInput",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.ToolArgInput",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.SessionIdInput",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.CwdInput",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.claude.v1.GitBranchInput",
        "Claude domain has no protojson fixtures yet",
    ),
    (
        "xuma.http.v1.HeaderInput",
        "HTTP domain has no protojson fixtures yet",
    ),
    (
        "xuma.http.v1.PathInput",
        "HTTP domain has no protojson fixtures yet",
    ),
    (
        "xuma.http.v1.MethodInput",
        "HTTP domain has no protojson fixtures yet",
    ),
    (
        "xuma.http.v1.QueryParamInput",
        "HTTP domain has no protojson fixtures yet",
    ),
    (
        "xuma.http.v1.AuthorityInput",
        "HTTP domain has no protojson fixtures yet",
    ),
    (
        "xuma.http.v1.SchemeInput",
        "HTTP domain has no protojson fixtures yet",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace root")
        .to_path_buf()
}

fn proto_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("read proto dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            out.extend(proto_files(&path));
        } else if path.extension().is_some_and(|e| e == "proto") {
            out.push(path);
        }
    }
    out
}

/// `package.Message` -> its field names, from the .proto sources.
fn schema() -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();

    for file in proto_files(&root().join("proto").join("xuma")) {
        let text = fs::read_to_string(&file).expect("read proto");
        let package = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("package ")?.strip_suffix(';'))
            .expect("every proto declares a package")
            .to_owned();

        let mut current: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim();

            if let Some(rest) = trimmed.strip_prefix("message ") {
                let name = rest.split_whitespace().next().expect("message name");
                current = Some(format!("{package}.{name}"));
                out.insert(current.clone().unwrap(), BTreeSet::new());
                continue;
            }
            if trimmed.starts_with('}') {
                current = None;
                continue;
            }
            let Some(msg) = current.as_ref() else {
                continue;
            };

            // `string name = 1;` or `map<string, string> metadata = 2;`
            if let Some((decl, _)) = trimmed.split_once('=') {
                let field = decl.split_whitespace().last().unwrap_or_default();
                if !field.is_empty() && field.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    out.get_mut(msg)
                        .expect("message present")
                        .insert(field.to_owned());
                }
            }
        }
    }
    out
}

/// `package.Message` -> the fields fixtures actually set on it.
fn fixtured() -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    fn visit(node: &serde_json::Value, out: &mut BTreeMap<String, BTreeSet<String>>) {
        match node {
            serde_json::Value::Array(items) => items.iter().for_each(|v| visit(v, out)),
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(url)) = map.get("@type") {
                    let name = url
                        .strip_prefix("type.googleapis.com/")
                        .unwrap_or(url)
                        .to_owned();
                    let entry = out.entry(name).or_default();
                    for key in map.keys().filter(|k| *k != "@type") {
                        entry.insert(key.clone());
                    }
                    // A payload body is not descended into: its keys belong to
                    // the payload, and nothing below it is an Any.
                    return;
                }
                map.values().for_each(|v| visit(v, out));
            }
            _ => {}
        }
    }

    let dir = root().join("spec").join("tests").join("07_protojson");
    for entry in fs::read_dir(&dir).expect("read fixtures") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|e| e != "yaml" && e != "yml") {
            continue;
        }
        let yaml = fs::read_to_string(&path).expect("read fixture");
        for doc in serde_yaml::Deserializer::from_str(&yaml) {
            let Ok(value) = serde_json::Value::deserialize(doc) else {
                continue;
            };
            if let Some(m) = value.get("proto_matcher") {
                visit(m, &mut out);
            }
        }
    }
    out
}

use serde::Deserialize;

#[test]
fn the_fixture_corpus_spans_the_schema() {
    let schema = schema();
    let seen = fixtured();
    let gaps: BTreeMap<&str, &str> = KNOWN_GAPS.iter().copied().collect();

    let mut problems: Vec<String> = Vec::new();

    for (message, fields) in &schema {
        let Some(hit) = seen.get(message) else {
            if !gaps.contains_key(message.as_str()) {
                problems.push(format!(
                    "{message}: no protojson fixture references it, and it is not a listed gap"
                ));
            }
            continue;
        };

        if gaps.contains_key(message.as_str()) {
            problems.push(format!(
                "{message}: listed as a known gap, but a fixture covers it — remove the entry"
            ));
        }

        for field in fields {
            // protojson writes lowerCamelCase; the schema is snake_case.
            let camel = {
                let mut s = String::with_capacity(field.len());
                let mut upper = false;
                for c in field.chars() {
                    match (c, upper) {
                        ('_', _) => upper = true,
                        (c, true) => {
                            s.extend(c.to_uppercase());
                            upper = false;
                        }
                        (c, false) => s.push(c),
                    }
                }
                s
            };
            if !hit.contains(field) && !hit.contains(&camel) {
                problems.push(format!("{message}.{field}: no fixture sets it"));
            }
        }
    }

    for (message, _) in KNOWN_GAPS {
        if !schema.contains_key(*message) {
            problems.push(format!(
                "{message}: listed as a known gap, but no such message exists"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "the fixture corpus does not span the schema:\n  {}\n\nEither add a protojson fixture, or \
         add the message to KNOWN_GAPS with a reason. That list is the visible form of the gap \
         and should only shrink.",
        problems.join("\n  ")
    );

    println!(
        "fixture coverage: {}/{} config messages fixtured, {} listed gaps",
        schema.len() - KNOWN_GAPS.len(),
        schema.len(),
        KNOWN_GAPS.len()
    );
}
