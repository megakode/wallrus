#!/usr/bin/env python3
"""Generate cargo-sources.json for Flatpak from Cargo.lock.

Reads Cargo.lock (v3 or v4) and produces a JSON array of flatpak-builder
source entries that vendor all crates.io dependencies offline.

Usage:
    python3 flatpak-cargo-generator.py [Cargo.lock] [-o cargo-sources.json]
"""

import json
import sys
import argparse

try:
    import toml
except ImportError:
    print("Error: 'toml' module required.  pip install toml", file=sys.stderr)
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("lockfile", nargs="?", default="Cargo.lock",
                        help="Path to Cargo.lock (default: Cargo.lock)")
    parser.add_argument("-o", "--output", default="cargo-sources.json",
                        help="Output JSON file (default: cargo-sources.json)")
    args = parser.parse_args()

    with open(args.lockfile) as f:
        lock = toml.load(f)

    sources = []
    for pkg in lock.get("package", []):
        name = pkg["name"]
        version = pkg["version"]
        source = pkg.get("source", "")
        checksum = pkg.get("checksum", "")

        # Skip non-registry packages (path deps, git deps, the root package)
        if not source or "registry" not in source:
            continue
        if not checksum:
            continue

        crate_dir = f"cargo/vendor/{name}-{version}"

        # Download and unpack the crate archive (.crate = tar.gz)
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
            "sha256": checksum,
            "strip-components": 1,
            "dest": crate_dir
        })

        # Create .cargo-checksum.json (required by cargo for vendored crates)
        sources.append({
            "type": "inline",
            "contents": json.dumps({"files": {}, "package": checksum}),
            "dest": crate_dir,
            "dest-filename": ".cargo-checksum.json"
        })

    print(f"Generated {len(sources) // 2} crate sources", file=sys.stderr)

    with open(args.output, "w") as f:
        json.dump(sources, f, indent=4)
        f.write("\n")

    print(f"Written to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
