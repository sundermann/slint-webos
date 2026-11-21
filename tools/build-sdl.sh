#!/usr/bin/env bash
set -euo pipefail
sdk=$1
build_dir=$2
tools_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
patch_file="$tools_dir/sdl-webos-relaunch.patch"
patch_key=$(sha256sum <"$patch_file")
revision=8f8da641e197c97a48c46c065e9a69941657bc29
checksum=407e8d00149df13753cc8b86639db527d569ef15c6dabc5b39f49d5b28579a7a
source_dir="$build_dir/SDL-webOS-$revision-${patch_key:0:12}"
# CMake records absolute paths to the relocated SDK and to the source tree.
sdk_key=$(printf '%s' "$sdk" | sha256sum)
variant="$build_dir/sdl-$revision-${patch_key:0:12}-${sdk_key:0:12}"
prefix="$variant/install"

# Keep stdout available for the resulting prefix and serialize use of the cache.
exec 3>&1 1>&2
exec 9>"$build_dir/.sdl-webos.lock"
flock 9

if [[ ! -f "$source_dir/CMakeLists.txt" ]]; then
    if [[ ${CARGO_NET_OFFLINE:-false} == true ]]; then
        echo "SDL-webOS source is missing from $build_dir. Build with network access once or set SDL_WEBOS_PREFIX." >&2
        exit 1
    fi

    staging=$(mktemp -d "$build_dir/.sdl-source.XXXXXX")
    trap 'rm -rf -- "$staging"' EXIT
    archive="$staging/sdl.tar.gz"
    curl --fail --location --proto '=https' --retry 3 --silent --show-error \
        "https://codeload.github.com/webosbrew/SDL-webOS/tar.gz/$revision" \
        --output "$archive"
    printf '%s  %s\n' "$checksum" "$archive" | sha256sum --check
    mkdir "$staging/source"
    tar -xzf "$archive" -C "$staging/source" --strip-components=1
    patch --batch -d "$staging/source" -p1 <"$patch_file"
    mv -T "$staging/source" "$source_dir"
fi

cmake -S "$source_dir" -B "$variant"     -DCMAKE_TOOLCHAIN_FILE="$sdk/share/buildroot/toolchainfile.cmake"     -DCMAKE_SYSTEM_NAME=Linux -DCMAKE_BUILD_TYPE=Release     -DCMAKE_INSTALL_PREFIX="$prefix" -DCMAKE_INSTALL_LIBDIR=lib     -DWEBOS=ON -DSDL_WAYLAND=ON -DSDL_WAYLAND_WEBOS=ON     -DSDL_X11=OFF -DSDL_WAYLAND_LIBDECOR=OFF -DSDL_AUDIO=OFF     -DSDL_JOYSTICK=OFF -DSDL_HAPTIC=OFF -DSDL_SENSOR=OFF     -DSDL_TEST=OFF -DSDL_SHARED=OFF -DSDL_STATIC=ON -DSDL_STATIC_PIC=ON
cmake --build "$variant" --parallel "${BUILD_JOBS:-${NUM_JOBS:-4}}"
cmake --install "$variant"
if ! grep -qx '#define SDL_VIDEO_DRIVER_WAYLAND_WEBOS 1' "$prefix/include/SDL2/SDL_config.h"; then
    echo "CMake did not enable the SDL Wayland webOS extension." >&2
    exit 1
fi
printf '%s\n' "$prefix" >&3
