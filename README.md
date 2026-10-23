[English](README-en.md) | 简体中文

# eunoia-mc-content-sync-cli

## 这是什么？

跨版本、加载器安装同模组的工具。

例如，现有 26.2 NeoForge 和 1.21.4 Fabric 实例，其中 26.2 NeoForge 安装了 Sodium，使用此工具可自动在 1.21.4 Fabric 实例安装符合版本需求的最新稳定版 Sodium。

不仅是模组，资源包、光影包也能自动、批量跨版本安装；已安装的模组、当前版本或加载器不支持的模组，也会附上提示。

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
