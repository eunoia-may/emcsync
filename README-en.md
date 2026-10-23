English | [简体中文](README.md)

# eunoia-mc-content-sync-cli

## What is this?

A tool for installing the same mods across Minecraft versions and loaders.

For example, suppose you have a 26.2 NeoForge instance and a 1.21.4 Fabric instance, and Sodium is installed in the former. This tool can automatically install the latest stable Sodium matching the target version into the 1.21.4 Fabric instance.

It is not limited to mods: resource packs and shader packs can also be installed automatically and in bulk across versions. Mods that are already installed, or unsupported by the current version or loader, are reported as well.

## Usage

```text
emcsync-cli sync <source_dir> <target_dir> -v <mc_version> -l <loader> -p <platform> [-y] [-d]
emcsync-cli help
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
| `-v, --version <mc_version>` | Minecraft version the resources belong to, for example `26.3`; the version list is not hardcoded and is fetched from upstream |
| `-l, --loader <loader>` | Loader: `fabric`, `forge`, `neoforge`, or `quilt` |
| `-p, --prefer <platform>` | Preferred platform to fetch resources from: `modrinth` or `curseforge` |
| `-y, --yes` | Assume yes and skip the confirmation prompt |
| `-d, --dry-run` | Show changes without installing |

An uppercase `-V, --version` is also available to show the program's own version.

## Roadmap

The following features are planned; the order does not reflect priority:

- Complete CurseForge resource support

- Allow choosing which resources not to sync

- A UI frontend (planned as a separate project)
