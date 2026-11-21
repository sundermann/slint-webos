#!/usr/bin/env python3
"""Package a verified ARM executable as a native webOS IPK."""

import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile

from verify_elf import inspect

TOOLS = Path(__file__).resolve().parent


def tar_gz(files):
    data = io.BytesIO()
    with gzip.GzipFile(fileobj=data, mode="wb", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT) as archive:
            for name, payload, mode in files:
                entry = tarfile.TarInfo(name)
                entry.size = len(payload)
                entry.mode = mode
                archive.addfile(entry, io.BytesIO(payload))
    return data.getvalue()


def write_ipk(output, members):
    with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as archive:
        path = Path(archive.name)
        try:
            archive.write(b"!<arch>\n")
            for name, data in members:
                header = f"{name + '/':<16}{0:<12}{0:<6}{0:<6}{'100644':<8}{len(data):<10}`\n"
                archive.write(header.encode("ascii"))
                archive.write(data)
                if len(data) % 2:
                    archive.write(b"\n")
            archive.close()
            path.chmod(0o644)
            path.replace(output)
        finally:
            path.unlink(missing_ok=True)


def package(args):
    info = json.loads(args.appinfo.read_text(encoding="utf-8"))
    if not isinstance(info, dict):
        raise ValueError("appinfo.json must contain an object")
    app_id, version, title = (info.get(key) for key in ("id", "version", "title"))
    if not isinstance(app_id, str) or not re.fullmatch(r"[a-z][a-z0-9]*(?:[.-][a-z0-9]+)+", app_id):
        raise ValueError("appinfo id must be a lowercase webOS application ID")
    if not isinstance(version, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("appinfo version must be major.minor.patch")
    if not isinstance(title, str) or not title.strip() or any(ord(ch) < 32 for ch in title):
        raise ValueError("appinfo title must be a non-empty line of text")
    if (info.get("type"), info.get("main"), info.get("icon")) != ("native", "app", "icon.png"):
        raise ValueError('appinfo must use type "native", main "app", and icon "icon.png"')
    if info.get("nativeLifeCycleInterfaceVersion") != 2:
        raise ValueError("appinfo must use nativeLifeCycleInterfaceVersion 2")

    icon = args.icon.read_bytes()
    if not icon.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("--icon must be a PNG")
    binary = args.binary.read_bytes()
    with tempfile.TemporaryDirectory(prefix="slint-webos-package-") as directory:
        staged = Path(directory) / args.binary.name
        staged.write_bytes(binary)
        verification = inspect(staged, args.readelf)
    verification["sha256"] = hashlib.sha256(binary).hexdigest()
    files = [
        ("appinfo.json", (json.dumps(info, indent=2) + "\n").encode(), 0o644),
        ("app", binary, 0o755),
        ("icon.png", icon, 0o644),
        ("build-info.json", (json.dumps(verification, indent=2) + "\n").encode(), 0o644),
        ("licenses/slint-webos.txt", (TOOLS.parent / "LICENSE").read_bytes(), 0o644),
        ("licenses/SDL-webOS.txt", (TOOLS / "licenses/SDL-webOS.txt").read_bytes(), 0o644),
    ]
    seen = {name for name, _, _ in files}
    for notice in args.notice:
        name = f"licenses/{notice.name}"
        if name in seen:
            raise ValueError(f"duplicate package entry {name}")
        seen.add(name)
        files.append((name, notice.read_bytes(), 0o644))

    control = (
        f"Package: {app_id}\nVersion: {version}\nSection: misc\nPriority: optional\n"
        f"Architecture: arm\nMaintainer: slint-webos\nDescription: {title}\n"
    ).encode()
    root = f"./usr/palm/applications/{app_id}/"
    args.output.mkdir(parents=True, exist_ok=True)
    output = args.output / f"{app_id}_{version}_arm.ipk"
    write_ipk(output, [
        ("debian-binary", b"2.0\n"),
        ("control.tar.gz", tar_gz([("./control", control, 0o644)])),
        ("data.tar.gz", tar_gz((root + name, data, mode) for name, data, mode in sorted(files))),
    ])
    print(output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--appinfo", type=Path, required=True)
    parser.add_argument("--icon", type=Path, required=True)
    parser.add_argument("--notice", type=Path, action="append", default=[])
    parser.add_argument("--output", type=Path, default=Path("dist"))
    parser.add_argument("--readelf", default="readelf")
    try:
        package(parser.parse_args())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"package: {error}\n")
