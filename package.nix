# packages.default 与 overlays 共用的唯一一份包定义，避免两处各写一遍后版本漂移。
{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "emcsync-cli";
  version = (lib.importTOML ./Cargo.toml).package.version;

  # 只收编译需要的文件；target、本地 cargo 缓存和测试实例都不进 store。
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./locales
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "复现 Minecraft 实例的内容资源";
    homepage = "https://github.com/eunoia-may/emcsync-cli";
    license = lib.licenses.mit;
    mainProgram = "emcsync-cli";
  };
}
