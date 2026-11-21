//! Build and link the native archives used for the webOS TV target.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("arm")
        || env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux")
    {
        return;
    }
    assert_eq!(
        env::var("TARGET").as_deref(),
        Ok("armv7-unknown-linux-gnueabi"),
        "webOS TV requires the ARMv7 soft-float ABI (armv7-unknown-linux-gnueabi)"
    );

    println!("cargo:rerun-if-env-changed=WEBOS_SDK");
    println!("cargo:rerun-if-env-changed=SDL_WEBOS_PREFIX");
    println!("cargo:rerun-if-changed=tools");
    let sdk =
        env::var_os("WEBOS_SDK").map_or_else(|| PathBuf::from("/opt/webos-sdk"), PathBuf::from);
    let sdk = sdk.canonicalize().unwrap_or_else(|error| {
        panic!(
            "set WEBOS_SDK to the relocated buildroot-nc4 SDK. {}: {error}",
            sdk.display()
        )
    });
    let sdk_lib_dir = sdk.join("arm-webos-linux-gnueabi/sysroot/usr/lib");
    let polyfills = sdk_lib_dir.join("libglibc_polyfills.a");
    assert!(polyfills.is_file(), "{} missing", polyfills.display());
    println!("cargo:rerun-if-changed={}", polyfills.display());

    let bundled_prefix = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo supplies CARGO_MANIFEST_DIR"),
    )
    .join("tools/sdl-webos");
    let prebuilt_prefix = env::var_os("SDL_WEBOS_PREFIX")
        .map(PathBuf::from)
        .or_else(|| bundled_prefix.is_dir().then_some(bundled_prefix));
    let using_prebuilt = prebuilt_prefix.is_some();
    let prefix = prebuilt_prefix.unwrap_or_else(|| build_sdl(&sdk));
    let sdl_lib_dir = prefix.join("lib");
    let sdl_archive = sdl_lib_dir.join("libSDL2.a");
    assert!(sdl_archive.is_file(), "{} missing", sdl_archive.display());
    assert!(
        !sdl_archive.with_extension("so").exists(),
        "use the static-only SDL-webOS install directory"
    );
    let config = prefix.join("include/SDL2/SDL_config.h");
    let header = fs::read_to_string(&config)
        .unwrap_or_else(|error| panic!("read {}: {error}", config.display()));
    assert!(
        header
            .lines()
            .any(|line| line == "#define SDL_VIDEO_DRIVER_WAYLAND_WEBOS 1"),
        "SDL2 was built without SDL_WAYLAND_WEBOS"
    );
    if using_prebuilt {
        println!("cargo:rerun-if-changed={}", sdl_archive.display());
        println!("cargo:rerun-if-changed={}", config.display());
    }

    // The Rust SDL bindings request SDL2 by name. Provide the static webOS
    // archive on their library search path and record its native dependencies.
    println!("cargo:rustc-link-search=native={}", sdl_lib_dir.display());
    println!("cargo:rustc-link-lib=static=SDL2");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-search=native={}", sdk_lib_dir.display());
    // libstd references getauxval after this crate appears on the linker line.
    println!("cargo:rustc-link-lib=static:+whole-archive,-bundle=glibc_polyfills");
}

fn build_sdl(sdk: &Path) -> PathBuf {
    println!("cargo:rerun-if-env-changed=BUILD_JOBS");
    println!("cargo:rerun-if-env-changed=SLINT_WEBOS_BUILD_DIR");
    let toolchain_file = sdk.join("share/buildroot/toolchainfile.cmake");
    println!("cargo:rerun-if-changed={}", toolchain_file.display());

    let out = env::var_os("SLINT_WEBOS_BUILD_DIR")
        .or_else(|| env::var_os("OUT_DIR"))
        .map(PathBuf::from)
        .expect("Cargo supplies OUT_DIR");
    fs::create_dir_all(&out).unwrap_or_else(|error| panic!("create {}: {error}", out.display()));
    let out = out
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve {}: {error}", out.display()));
    let build = Command::new("bash")
        .arg("tools/build-sdl.sh")
        .arg(sdk)
        .arg(out)
        .output()
        .expect("run tools/build-sdl.sh with Bash");
    assert!(
        build.status.success(),
        "building SDL-webOS failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let prefix = String::from_utf8(build.stdout).expect("SDL-webOS installation path is UTF-8");
    PathBuf::from(prefix.trim_end())
}
