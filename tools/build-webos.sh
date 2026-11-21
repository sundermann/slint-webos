#!/usr/bin/env bash
set -euo pipefail

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if (( $# == 0 )); then
    cd "$repo"
    set -- --locked --example counter
fi

# shellcheck source=webos-env.sh
source "$repo/tools/webos-env.sh"
exec cargo build --release --target armv7-unknown-linux-gnueabi "$@"
