#!/usr/bin/env bash
#
# Build and test each feature of rumi-http in isolation.
#
# Everything was `--all-features` only, which is exactly why a feature split
# could ship broken: `--features gateway` did not compile, and every test of the
# now-gateway-only code was written against the ext_proc constructor. A split
# that CI does not build decays back into conflation within two commits.
#
# It also catches the additivity violation this repo has produced three times:
# a feature that REMOVES an item rather than adding one shows up here as a
# combination that fails while a superset passes.
set -uo pipefail

cd "$(dirname "$0")/.."

# There is no `proto` feature: with one config vocabulary there is nothing to
# select, so a `not(feature = "proto")` cfg is inexpressible.
COMBOS=(
    ""
    "gateway"
    "ext-proc"
    "registry"
    "registry,gateway"
    "registry,ext-proc"
    "gateway,ext-proc"
)

failed=0
for combo in "${COMBOS[@]}"; do
    label="${combo:-<none>}"
    if [ -z "$combo" ]; then
        args=(--no-default-features)
    else
        args=(--no-default-features --features "$combo")
    fi

    if cargo test -q -p rumi-http --manifest-path rumi/Cargo.toml "${args[@]}" >/dev/null 2>&1; then
        printf '  ok    %s\n' "$label"
    else
        printf '  FAIL  %s\n' "$label"
        failed=$((failed + 1))
    fi
done

# --all-features must work too.
if cargo test -q -p rumi-http --manifest-path rumi/Cargo.toml --all-features >/dev/null 2>&1; then
    printf '  ok    <all>\n'
else
    printf '  FAIL  <all>\n'
    failed=$((failed + 1))
fi

echo
if [ "$failed" -gt 0 ]; then
    echo "check-features: $failed combination(s) failed"
    exit 1
fi
echo "check-features: every feature combination builds and tests clean"
