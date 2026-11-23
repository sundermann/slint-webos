#!/usr/bin/env bash
set -euo pipefail
sdk=$1
build_dir=$2
revision=c7bf0a722739ac734a97f66105f0fbf1cc56443c
checksum=c2860318ab5a6cf7d0676c3d8f14ba99da19b01b4f5c14ab6d95b1b3612342e4
source_dir="$build_dir/SDL-webOS-$revision"
# CMake records absolute paths to the relocated SDK and to the source tree.
sdk_key=$(printf '%s' "$sdk" | sha256sum)
variant="$build_dir/sdl-$revision-${sdk_key:0:12}"
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
