English | [简体中文](README.md)

> **Note:** This document is a non-Chinese version of the README and is translated by AI; some parts may be inaccurate. If you find any problems, please [open an issue](https://github.com/eunoia-may/emcsync/issues) to let us know.

# emcsync

## What is this?

A tool for installing the same mods across Minecraft versions and loaders.

For example, suppose you have a 26.2 NeoForge instance and a 1.21.4 Fabric instance, and Sodium is installed in the former. This tool can automatically install the latest stable Sodium matching the target version into the 1.21.4 Fabric instance.

It is not limited to mods: resource packs and shader packs can also be installed automatically and in bulk across versions. Mods that are already installed, or unsupported by the current version or loader, are reported as well.

## Installation

### Prebuilt binaries

Download the archive for your platform from [Releases](https://github.com/eunoia-may/emcsync/releases):

| Platform | File |
| --- | --- |
| Linux x86_64 | `emcsync-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `emcsync-<version>-aarch64-unknown-linux-musl.tar.gz` |
| Windows x86_64 | `emcsync-<version>-x86_64-pc-windows-msvc.zip` |
| Windows aarch64 | `emcsync-<version>-aarch64-pc-windows-msvc.zip` |
| macOS x86_64 | `emcsync-<version>-x86_64-apple-darwin.tar.gz` |
| macOS aarch64 | `emcsync-<version>-aarch64-apple-darwin.tar.gz` |

On Linux and macOS, unpack the archive, run `chmod +x emcsync`, and put it on your `PATH`. The macOS builds are unsigned, so Gatekeeper blocks the first run: allow it under System Settings → Privacy & Security.

On Windows, unpacking to any directory is the installation; deleting the directory removes it. Configs and caches live inside the game instance directory, leaving nothing global behind.

### NixOS and home-manager

Add the input in your `flake.nix`:

```nix
inputs.emcsync.url = "github:eunoia-may/emcsync";
```

Reference the module in your module list. The project only puts the package into `pkgs`; you need to declare the package to install it:

```nix
# NixOS: one line in nixosConfigurations.<host>.modules
inputs.emcsync.nixosModules.default

# home-manager: one line in the imports of home.nix or flake.nix
inputs.emcsync.homeModules.default
```

Then list the package name:

```nix
environment.systemPackages = with pkgs; [ emcsync ];  # home.packages for home-manager
```

If you prefer no overlay, you can also use `inputs.emcsync.packages.${pkgs.stdenv.hostPlatform.system}.default` to install a fixed build.

### From source

Requires Rust 1.99 or newer, or just `nix develop`:

```sh
cargo build --release
# the binary lands in target/release/emcsync
```

## Usage

```text
emcsync sync <source_dir> <target_dir> -v <mc_version> -l <loader> [-j <jobs>] [-y] [-d]
emcsync help
```

`help`, `-h`, and `--help` all print the help.

### Commands

| Command | Description |
| --- | --- |
| `sync` | Sync resources |
| `help` | Print help |

### Arguments and options

| Parameter | Description |
| --- | --- |
| `<source_dir>` | Source instance directory; quote it with `""` or `''` if the path contains spaces |
| `<target_dir>` | Target instance directory; quote it with `""` or `''` if the path contains spaces |
| `-v, --version <mc_version>` | Minecraft version the resources to reproduce belong to, for example `26.3` |
| `-l, --loader <loader>` | Loader: `fabric`, `forge`, `neoforge`, or `quilt` |
| `-j, --jobs <jobs>` | Number of files to download at once, 1-16, default 4 |
| `-y, --yes` | Assume yes and skip the confirmation prompt |
| `-d, --dry-run` | Show changes without installing |

An uppercase `-V, --version` is also available to show the program's own version.

## Notes

- **CurseForge support**: This project is **self-use first**. Modrinth resources already cover my needs, and downloading from CurseForge is rather troublesome, so it is not supported for now.

- **No external pull requests**: This project is developed with AI assistance, and the author's coding skills are modest, which leaves me unable to review pull requests; therefore the PR channel is closed. If you find a bug, feel free to report it via [Issues](https://github.com/eunoia-may/emcsync/issues).
