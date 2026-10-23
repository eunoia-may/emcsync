# 人性化纠错

只是当下能搜索到的最主流与合适的做法，后续如有需要应重新审阅、搜索，并在需要时修改执行流。

## 大小写

`loader` 与 `platform` 都用 clap 的 `ignore_case`，`Fabric`、`MODRINTH` 都能认。

## 拼写

拼错时用 clap 的 `suggestions` 给出最接近的候选，不要直接甩 usage。

帮助的三种写法见 `规范-命令设计与简单演示.md`。
