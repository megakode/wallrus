# Flatpak Release Process

## Prerequisites

- Push access to `github.com/megakode/wallrus` (app repo)
- Push access to `github.com/flathub/io.github.megakode.Wallrus` (Flathub repo)

## Steps

### 1. Bump the version in the app repo

Update the version in `Cargo.toml`:

```toml
version = "X.Y.Z"
```

### 2. Add a release entry to metainfo

Add a new `<release>` block at the top of the `<releases>` section in
`data/io.github.megakode.Wallrus.metainfo.xml`:

```xml
<releases>
    <release version="X.Y.Z" date="YYYY-MM-DD">
      <description>
        <p>What changed in this release.</p>
      </description>
    </release>
    <!-- older releases below -->
</releases>
```

### 3. Update the Nix package version (if applicable)

In `nix/package.nix`:

```nix
version = "X.Y.Z";
```

### 4. Commit, tag, and push

```bash
git add Cargo.toml data/io.github.megakode.Wallrus.metainfo.xml nix/package.nix
git commit -m "Release vX.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main --tags
```

### 5. Update the Flathub repo

Clone or update your local copy of the Flathub repo:

```bash
git clone git@github.com:flathub/io.github.megakode.Wallrus.git
cd io.github.megakode.Wallrus
```

Update the manifest source tag:

```json
{
    "type": "git",
    "url": "https://github.com/megakode/wallrus.git",
    "tag": "vX.Y.Z"
}
```

If dependencies changed, regenerate `cargo-sources.json` from the app repo:

```bash
python3 /path/to/wallrus/flatpak/flatpak-cargo-generator.py /path/to/wallrus/Cargo.lock -o cargo-sources.json
```

Commit and push:

```bash
git add io.github.megakode.Wallrus.json cargo-sources.json
git commit -m "Update to vX.Y.Z"
git push
```

Flathub CI will build and publish the new version automatically.

## Version locations

| File | What to update |
|------|----------------|
| `Cargo.toml` | `version = "X.Y.Z"` (source of truth, read at build time via `env!("CARGO_PKG_VERSION")`) |
| `data/io.github.megakode.Wallrus.metainfo.xml` | New `<release>` entry (shown on Flathub store page) |
| `nix/package.nix` | `version = "X.Y.Z"` |
| Flathub repo manifest | `"tag": "vX.Y.Z"` (tells Flathub which commit to build) |
