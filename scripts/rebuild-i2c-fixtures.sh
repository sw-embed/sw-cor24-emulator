#!/usr/bin/env bash
# Rebuild every committed I2C demo fixture and report drift.
#
# Skips with exit 0 if tc24r is not on PATH (so it's safe to run in any
# environment). When tc24r is present, runs `make clean && make` in each
# examples/i2c/*/ directory and `git diff --exit-code` against the
# committed .lgo, surfacing any drift.

if ! command -v tc24r >/dev/null 2>&1; then
    echo "tc24r not on PATH — skipping fixture rebuild."
    exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel)" || exit 1
cd "$REPO_ROOT" || exit 1

drift=0
for dir in examples/i2c/*/; do
    [[ -f "${dir}Makefile" ]] || continue
    echo "=== $dir ==="
    if ! (cd "$dir" && make clean && make); then
        echo "BUILD FAILED in $dir"
        drift=1
        continue
    fi
    if ! git diff --exit-code -- "${dir}"*.lgo; then
        echo "DRIFT in $dir"
        drift=1
    fi
done

if [[ $drift -ne 0 ]]; then
    echo "FIXTURES DRIFTED — review and commit if intentional."
fi
exit $drift
