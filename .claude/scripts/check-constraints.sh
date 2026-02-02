#!/bin/bash
# Validates all arch-guild constraints

set -e

XUMA_ROOT="${1:-$(pwd)}"

if [[ ! -d "$XUMA_ROOT/rumi" ]]; then
    echo "Error: Not in x.uma directory (rumi/ not found)"
    exit 1
fi

echo "Checking arch-guild constraints in $XUMA_ROOT..."

# 1. ReDoS Protection - no fancy-regex
echo -n "1. ReDoS Protection... "
if grep -rq "fancy.regex" "$XUMA_ROOT/rumi/" 2>/dev/null; then
    echo "FAIL: fancy-regex detected"
    exit 1
fi
echo "OK"

# 2. Max depth constant exists
echo -n "2. Max Depth Constant... "
if ! grep -rq "MAX.*DEPTH" "$XUMA_ROOT/rumi/src/" 2>/dev/null; then
    echo "WARN: MAX_DEPTH constant not found (may be implemented differently)"
else
    echo "OK"
fi

# 3. Type registry immutable (no &mut self on registry)
echo -n "3. Type Registry Immutable... "
if grep -rqE "impl.*Registry.*\{" "$XUMA_ROOT/rumi/" 2>/dev/null; then
    if grep -rqE "&mut\s+self" "$XUMA_ROOT/rumi/src/" 2>/dev/null | grep -qi "registry"; then
        echo "WARN: Registry may have mutable methods"
    else
        echo "OK"
    fi
else
    echo "OK (no registry impl found yet)"
fi

# 4. Send + Sync + Debug (check for marker tests)
echo -n "4. Send + Sync + Debug... "
if grep -rq "assert_send_sync" "$XUMA_ROOT/rumi/" 2>/dev/null || \
   grep -rq "Send + Sync" "$XUMA_ROOT/rumi/src/" 2>/dev/null; then
    echo "OK"
else
    echo "WARN: No Send+Sync marker tests found"
fi

# 5. No recursive evaluate calls (check for explicit stack)
echo -n "5. Iterative Evaluation... "
RECURSIVE_CALLS=$(grep -rn "\.evaluate(" "$XUMA_ROOT/rumi/src/" 2>/dev/null | wc -l)
if [ "$RECURSIVE_CALLS" -gt 5 ]; then
    echo "WARN: Multiple evaluate() calls found - verify they're not recursive"
else
    echo "OK"
fi

# 6. OnMatch is enum (not struct with Option fields)
echo -n "6. OnMatch Exclusivity... "
if grep -rq "enum OnMatch" "$XUMA_ROOT/rumi/src/" 2>/dev/null; then
    echo "OK"
elif grep -rq "struct OnMatch" "$XUMA_ROOT/rumi/src/" 2>/dev/null; then
    echo "FAIL: OnMatch should be enum, not struct"
    exit 1
else
    echo "OK (OnMatch not implemented yet)"
fi

# 7. Action bounds include 'static
echo -n "7. Action 'static... "
if grep -rqE "Output:\s*'static" "$XUMA_ROOT/rumi/src/" 2>/dev/null || \
   grep -rqE "A:\s*.*'static" "$XUMA_ROOT/rumi/src/" 2>/dev/null; then
    echo "OK"
else
    echo "WARN: 'static bound not found on action types"
fi

# 8-9. Clone + Send + Sync bounds
echo -n "8-9. Action Clone+Send+Sync... "
if grep -rqE "Clone\s*\+\s*Send\s*\+\s*Sync" "$XUMA_ROOT/rumi/src/" 2>/dev/null; then
    echo "OK"
else
    echo "WARN: Clone+Send+Sync bounds not found"
fi

echo ""
echo "Constraint check complete!"
