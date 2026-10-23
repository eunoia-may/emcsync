[English](README-en.md) | 简体中文

# eunoia-mc-content-sync-cli

## 这是什么？

跨版本、加载器安装同模组的工具。

例如，现有 26.2 NeoForge 和 1.21.4 Fabric 实例，其中 26.2 NeoForge 安装了 Sodium，使用此工具可自动在 1.21.4 Fabric 实例安装符合版本需求的最新稳定版 Sodium。

不仅是模组，资源包、光影包也能自动、批量跨版本安装；已安装的模组、当前版本或加载器不支持的模组，也会附上提示。

## 安装

### 预编译产物

从 [Releases](https://github.com/eunoia-may/emcsync-cli/releases) 下载对应平台的一份：

| 平台 | 文件 |
| --- | --- |
| Linux x86_64 | `emcsync-cli-<版本>-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `emcsync-cli-<版本>-aarch64-unknown-linux-musl.tar.gz` |
| Windows x86_64 | `emcsync-cli-<版本>-x86_64-pc-windows-msvc.zip` |
| Windows aarch64 | `emcsync-cli-<版本>-aarch64-pc-windows-msvc.zip` |
| macOS x86_64 | `emcsync-cli-<版本>-x86_64-apple-darwin.tar.gz` |
| macOS aarch64 | `emcsync-cli-<版本>-aarch64-apple-darwin.tar.gz` |

Linux 与 macOS 解压后 `chmod +x emcsync-cli`，放进 `PATH` 即可。macOS 产物未签名，首次运行会被 Gatekeeper 拦一次，到“系统设置 → 隐私与安全性”里放行。

Windows 解压到任意目录即是安装，删掉目录即是卸载；配置与缓存都写在游戏实例目录里，不留全局痕迹。

### NixOS 与 home-manager

在 `flake.nix` 的 `inputs` 里加：

```nix
inputs.emcsync-cli.url = "github:eunoia-may/emcsync-cli";
```

在模块列表里引用本项目模块，项目只把包挂进 `pkgs`，需主动声明包来安装：

```nix
# NixOS：flake.nix 里 nixosConfigurations.<主机>.modules 中加一行
inputs.emcsync-cli.nixosModules.default

# home-manager：home.nix 或 flake.nix 的 imports 里加一行
inputs.emcsync-cli.homeModules.default
```

然后在包列表里写包名：

```nix
environment.systemPackages = with pkgs; [ emcsync-cli ];  # home-manager 用 home.packages
```

不想用 overlay 也可以直接写 `inputs.emcsync-cli.packages.${pkgs.stdenv.hostPlatform.system}.default`，等同于装一份固定的包。

### 从源码

需要 Rust 1.99 以上，或直接用 `nix develop` 进环境：

```sh
cargo build --release
# 产物在 target/release/emcsync-cli
```

## 用法

```text
emcsync-cli sync <source_dir> <target_dir> -v <mc_version> -l <loader> -p <platform> [-y] [-d]
emcsync-cli help
```

`help`、`-h`、`--help` 均可输出帮助。

### 命令

| 命令 | 说明 |
| --- | --- |
| `sync` | 同步资源 |
| `help` | 显示帮助 |

### 参数说明

| 参数 | 说明 |
| --- | --- |
| `<source_dir>` | 提取源，即原实例的目录；路径含空格时用引号（`""` 或 `''`）包裹 |
| `<target_dir>` | 复现目标，即待复现实例的目录；路径含空格时用引号（`""` 或 `''`）包裹 |
| `-v, --version <mc_version>` | 要复现的资源所属的 Minecraft 版本，例如 `26.3`；版本列表不硬编码，从上游获取 |
| `-l, --loader <loader>` | 加载器，可选 `fabric`、`forge`、`neoforge`、`quilt` |
| `-p, --prefer <platform>` | 优先从哪个平台获取资源，可选 `modrinth`、`curseforge` |
| `-y, --yes` | 预同意，跳过询问环节 |
| `-d, --dry-run` | 只展示变化，不实际安装 |

另有大写 `-V, --version`，用于显示程序自身的版本。

## 后续计划
打算补全这些功能，排序不代表优先级：

- 补全 CurseForge 资源的支持

- 允许勾选哪些资源不同步

- UI 前端（计划新开项目）
