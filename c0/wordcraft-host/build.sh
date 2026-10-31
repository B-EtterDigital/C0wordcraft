#!/usr/bin/env bash
# The C0X payload builder runs a page's own build.sh when it has one (scripts/sow/upstream/build-payload.mjs), so the
# size-tuned page build (build-page.sh) is what ships. A build.sh page admits itself to a heavy slot, like the builder does.
set -euo pipefail
cd "$(dirname "$0")"
exec smoac-slot run heavy 2 3600 -- env CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" BINARYEN_CORES="${CARGO_BUILD_JOBS:-4}" bash ./build-page.sh "$@"
