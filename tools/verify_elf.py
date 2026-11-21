#!/usr/bin/env python3
"""Verify that a release executable uses the webOS ARM ABI and both static archives."""

import argparse
import os
from pathlib import Path
import re
import subprocess

TARGET = "armv7-unknown-linux-gnueabi"


def inspect(binary, readelf):
    command = [readelf, "-W", "-h", "-l", "-d", "-s", "-V", "-A", str(binary.resolve())]
    elf = subprocess.check_output(command, env={**os.environ, "LC_ALL": "C"}, text=True)
    if not all(re.search(pattern, elf, re.MULTILINE) for pattern in (
        r"^\s*Class:\s+ELF32$",
        r"^\s*Data:\s+.*little endian$",
        r"^\s*Machine:\s+ARM$",
        r"^\s*Tag_CPU_arch:\s+v7$",
        r"^\s*Flags:.*Version5 EABI, soft-float ABI",
        r"\[Requesting program interpreter: /lib/ld-linux.so.3\]",
    )):
        raise ValueError(f"{binary} must be a buildroot-nc4 ARMv7 soft-float executable")

    dependencies = re.findall(r"\(NEEDED\).*\[([^\]]+)\]", elf)
    if any(re.match(r"lib(?:SDL2|glibc_polyfills)", name, re.I) for name in dependencies):
        raise ValueError(f"{binary} imports a shared SDL2 or glibc polyfills library")

    defined = set()
    for line in elf.splitlines():
        fields = line.split()
        if len(fields) >= 8 and fields[0].endswith(":") and fields[0][:-1].isdigit():
            if fields[6].isdigit():
                defined.add(fields[7])
    for symbol in ("SDL_Init", "WaylandWebOS_VideoInit", "getauxval"):
        if symbol not in defined:
            raise ValueError(f"{binary} lacks {symbol}; verify the release build before stripping symbols")

    runtime_paths = re.findall(r"\((?:RPATH|RUNPATH)\).*\[([^\]]*)\]", elf)
    versions = {tuple(map(int, name.split("."))) for name in re.findall(r"\bGLIBC_(\d+(?:\.\d+)+)\b", elf)}
    return {
        "target": TARGET,
        "shared_dependencies": sorted(dependencies),
        "runtime_library_paths": runtime_paths,
        "glibc_symbol_versions": [".".join(map(str, version)) for version in sorted(versions)],
        "static_symbols": ["SDL_Init", "WaylandWebOS_VideoInit", "getauxval"],
    }


if __name__ == "__main__":
    import json

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--readelf", default="readelf")
    args = parser.parse_args()
    try:
        print(json.dumps(inspect(args.binary, args.readelf), indent=2))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"verify-elf: {error}\n")
