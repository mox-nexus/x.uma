#!/usr/bin/env bash
#
# Regenerate proto code for all three languages.
#
# Lives in a script rather than inline in the justfile so CI can run the exact
# same steps without installing `just` — the setup action could not resolve a
# release and took the codegen job down twice. `just gen` calls this, so there
# is still one source.
#
# Two passes are required. `buf generate` only walks the local module graph, and
# no xuma proto imports xDS, so the xds.* types convert.rs depends on are absent
# from it. The second pass fetches them explicitly, scoped with --path: without
# it, buf pulls the whole cncf/xds module — ORCA load-reporting services and
# annotation metadata, ~4,500 lines nothing compiles.
#
# Both passes generate into a staging tree and swap only once both succeed.
# buf.gen.yaml carries `clean: true`, so an in-place generate that fails
# part-way leaves the committed tree destroyed — and the second pass reaches the
# network, which rate-limits. Observed on 2026-08-17.
#
# Nothing here deletes: the outgoing trees are moved into the staging directory
# and left for the OS to reap, so a bad swap stays recoverable.
set -euo pipefail

cd "$(dirname "$0")/.."

TREES=(rumi/proto/src/gen puma/proto/src/gen bumi/proto/src/gen)

STAGE=$(mktemp -d)
echo "gen: staging in $STAGE"

buf generate -o "$STAGE"
buf generate buf.build/cncf/xds --template buf.gen.rust.yaml -o "$STAGE" \
    --path xds/core/v3 --path xds/type/v3 --path xds/type/matcher/v3

for d in "${TREES[@]}"; do
    test -d "$STAGE/$d" || { echo "gen: $d missing from staging, refusing to swap" >&2; exit 1; }
done

mkdir -p "$STAGE/.outgoing"
for d in "${TREES[@]}"; do
    if [ -d "$d" ]; then mv "$d" "$STAGE/.outgoing/${d//\//_}"; fi
    mkdir -p "$(dirname "$d")"
    mv "$STAGE/$d" "$d"
done

echo "gen: ok — previous trees kept in $STAGE/.outgoing"
