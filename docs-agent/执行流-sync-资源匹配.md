# 资源匹配

只是当下能搜索到的最主流与合适的做法，后续如有需要应重新审阅、搜索，并在需要时修改执行流。

当前只做 Modrinth。CurseForge 相关的内容按设计保留，暂不实现。

## 扫描

分别扫描源目录和目标目录下的 `mods`、`resourcepacks`、`shaderpacks` 三个文件夹，收集所有 `.jar` 和 `.zip` 文件。不递归子目录，跳过隐藏文件。对每个文件计算两种哈希：小写的 SHA-1（给 Modrinth 用）和 MurmurHash2（给 CurseForge 用，计算前剥离空白字符，seed 为 1）。

资源类型由所在目录决定：`mods` → `mod`，`resourcepacks` → `resourcepack`，`shaderpacks` → `shaderpack`。

## 反查

源和目标走同一套逻辑。优先用 `--prefer` 指定的平台反查。

兜底的含义是：**换哈希算法查另一个平台**。Modrinth 未命中的文件，拿它的 MurmurHash2 指纹去 CurseForge 查；反过来同理。不要拿同一个哈希值跨平台乱发。

两个平台的小脾气：
- Modrinth 要求请求带一个唯一的 `User-Agent`，不然可能被拦。
- CurseForge 的指纹查询要带上 `gameId=432`。

反查成功的结果记录 `project_id` 和平台名。两个平台都查不到的哈希，对应的文件归入**“无法匹配”**列表（记录文件名），不参与后续的差集比对。

## 差集

拿源里的 `project_id` 集合，减去目标里的 `project_id` 集合。剩下的就是目标缺失的资源（待安装）。交集部分归入“已存在”，不进入下一步。

## 查兼容版本

对差集中每个资源，拿 `--version` 去平台查最新稳定版。加载器只对模组生效：

- `mod`：同时按 `game_versions` 和 `loaders` 过滤。
- `resourcepack`、`shaderpack`：只按 `game_versions` 过滤。它们的版本里 `loaders` 是 `minecraft` 或 `iris`/`optifine`，带上 `--loader` 会全部查空。

只保留 `version_type == "release"`，按 `date_published` 取最新。

CurseForge 这边还要多看一眼：部分模组禁用了第三方下载，`downloadUrl` 会返回 `null`。这种情况直接归入“此版本不存在”，不要留到下载阶段才炸。

查不到的归入“此版本不存在”，能查到的进入待安装列表。

## 输出分类

四类展示：
- **待安装**：显示 `<slug> (<类型>) (<平台>)`。
- **此版本不存在**：显示 `<slug> (<类型>)`，不显示平台（省去额外反查）。
- **已存在**：显示 `<slug> (<类型>)`，不显示平台。
- **无法匹配**：直接显示原始文件名，如 `sjaks.jar`。这些文件不参与差集，也不影响同步结果，只是告知用户“已忽略”。

每类内部按名称正向排序。

## 安装

按资源类型落回目标目录对应的子目录，文件名用平台返回的 `filename`：

- `mod` → `mods/`
- `resourcepack` → `resourcepacks/`
- `shaderpack` → `shaderpacks/`

子目录缺失时随安装一并创建，不单独提问：缺了哪些写进确认提示，与安装确认合成一次询问。带 `-y` 直接创建，`--dry-run` 只提示不创建。目标实例目录本身不存在则直接报错退出。

先写 `<filename>.part` 再改名，下载中断不会留下半个文件。非 tty 且没带 `-y` 时不能弹询问，直接报错提示加 `-y`。
