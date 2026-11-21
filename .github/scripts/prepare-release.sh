#!/usr/bin/env bash
set -euo pipefail

version=$1
release_dir=$2
target=armv7-unknown-linux-gnueabi
bundle=tools/sdl-webos

mkdir -p build/release-sdl
prefix=$(bash tools/build-sdl.sh "$WEBOS_SDK" "$(realpath build/release-sdl)")
install -Dm644 "$prefix/lib/libSDL2.a" "$bundle/lib/libSDL2.a"
install -Dm644 "$prefix/include/SDL2/SDL_config.h" "$bundle/include/SDL2/SDL_config.h"

cargo package --locked --allow-dirty --no-verify --target-dir "$release_dir/package"
tar -xzf "$release_dir/package/package/slint-webos-$version.crate" -C "$release_dir"
crate_dir="$release_dir/slint-webos-$version"
cmp "$bundle/lib/libSDL2.a" "$crate_dir/$bundle/lib/libSDL2.a"
cmp "$bundle/include/SDL2/SDL_config.h" "$crate_dir/$bundle/include/SDL2/SDL_config.h"
test -s "$crate_dir/tools/licenses/SDL-webOS.txt"

app_dir="$release_dir/app"
mkdir -p "$app_dir/src"
cp "$crate_dir/examples/counter.rs" "$app_dir/src/main.rs"
cp "$crate_dir/Cargo.lock" "$app_dir/Cargo.lock"
cat > "$app_dir/Cargo.toml" <<EOF
[package]
name = "slint-webos-release-check"
version = "0.0.0"
edition = "2024"

[dependencies]
slint = { version = "1.19", default-features = false, features = ["std", "compat-1-2"] }
slint-webos = { path = "../slint-webos-$version" }
EOF

cargo fetch --manifest-path "$app_dir/Cargo.toml" --target "$target"
