# Release packaging

This plugin follows the prebuilt-binary install pattern used by
`persiyanov/herdr-reviewr`:

| Area | Pattern |
| --- | --- |
| Herdr build step | `[[build]]` runs one install script: `bash herdr/install.sh`. |
| Release assets | GitHub Actions builds one tarball per target and uploads a `.sha256` sidecar. |
| Install path | The install script extracts the verified binary into the plugin's `bin/` directory. |
| Runtime path | Manifest commands execute `$HERDR_PLUGIN_ROOT/bin/<binary>` through `sh -c`. |

For this plugin, the simplest matching design is:

- one Herdr install script: `herdr/install.sh`;
- GitHub Actions build release archives for Linux and macOS;
- `herdr/install.sh` downloads the matching release archive and SHA-256 sidecar;
- the verified binary is installed at `bin/herdr-tab-smart-rename-rs`;
- manifest actions and events call `$HERDR_PLUGIN_ROOT/bin/herdr-tab-smart-rename-rs`.

This keeps local development and GitHub installation aligned:

```sh
cargo build --release
mkdir -p bin
install -m 0755 target/release/herdr-tab-smart-rename-rs bin/herdr-tab-smart-rename-rs
herdr plugin link .
```

```sh
herdr plugin install EmmetZ/herdr-tab-smart-rename-rs
```

The release repository is pinned in `herdr/install.sh` as
`EmmetZ/herdr-tab-smart-rename-rs`, matching the `herdr-reviewr` style.
