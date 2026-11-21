#!/usr/bin/env bash
# Source this file on a Linux host to configure Cargo for the webOS SDK.
# Run the setup in a function so sourcing does not alter the caller's shell options.
_slint_webos_setup() {
    local sdk compiler prefix build_dir
    sdk=$(realpath "${WEBOS_SDK:-/opt/webos-sdk}") || return 1
    compiler="$sdk/bin/arm-webos-linux-gnueabi-gcc"
    if [[ ! -x "$compiler" || ! -f "$sdk/share/buildroot/toolchainfile.cmake" ]]; then
        echo "Set WEBOS_SDK to the relocated buildroot-nc4 2026.08-webos SDK." >&2
        return 1
    fi

    if [[ -n "${SDL_WEBOS_PREFIX:-}" ]]; then
        prefix=$(realpath "$SDL_WEBOS_PREFIX") || return 1
        export SDL_WEBOS_PREFIX="$prefix"
    fi
    if [[ -n "${SLINT_WEBOS_BUILD_DIR:-}" ]]; then
        mkdir -p "$SLINT_WEBOS_BUILD_DIR" || return 1
        build_dir=$(realpath "$SLINT_WEBOS_BUILD_DIR") || return 1
        export SLINT_WEBOS_BUILD_DIR="$build_dir"
    fi

    export WEBOS_SDK="$sdk"
    export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER="$compiler"
    export CC_armv7_unknown_linux_gnueabi="$compiler"
    export AR_armv7_unknown_linux_gnueabi="$sdk/bin/arm-webos-linux-gnueabi-ar"
}

if _slint_webos_setup; then
    unset -f _slint_webos_setup
else
    unset -f _slint_webos_setup
    return 1 2>/dev/null || exit 1
fi
