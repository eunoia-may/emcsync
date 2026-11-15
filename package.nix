# packages.default 与 overlays 共用的唯一一份包定义，避免两处各写一遍后版本漂移。
{ lib, stdenvNoCC, fetchurl }:

let
  version = (lib.importTOML ./Cargo.toml).package.version;

  # 对应 .github/workflows/release.yml 的产物，与 flake 支持的 system 一一对应。
  # 发新版本时换掉三个 hash 即可。
  sources = {
    x86_64-linux = {
      target = "x86_64-unknown-linux-musl";
      hash = "sha256-u/hEiqKGiIEOKnc7e73D3gqSzm/E8TVynuBORjyRcJo=";
    };
    aarch64-linux = {
      target = "aarch64-unknown-linux-musl";
      hash = "sha256-evLhJsGLzZhYxLAfNHP0f1X1v5HAQs80Ek2bXZZNMlc=";
    };
    aarch64-darwin = {
      target = "aarch64-apple-darwin";
      hash = "sha256-uGu/uO38PkZroJqRnmcL10lL0uLvq5AmKHJFjANEmug=";
    };
  };

  system = stdenvNoCC.hostPlatform.system;
  source = sources.${system} or (throw "emcsync 未提供 ${system} 的预编译产物");

  dirName = "emcsync-${version}-${source.target}";
in
stdenvNoCC.mkDerivation {
  pname = "emcsync";
  inherit version;

  src = fetchurl {
    url = "https://github.com/eunoia-may/emcsync/releases/download/v${version}/${dirName}.tar.gz";
    inherit (source) hash;
  };

  # 产物已是 musl 静态链接或原生 macOS 二进制，无需修补，也不需要 strip。
  dontStrip = true;
  dontBuild = true;

  sourceRoot = dirName;

  installPhase = ''
    install -Dm755 emcsync $out/bin/emcsync
    install -Dm644 LICENSE $out/share/licenses/emcsync/LICENSE
  '';

  meta = {
    description = "复现 Minecraft 实例的内容资源";
    homepage = "https://github.com/eunoia-may/emcsync";
    license = lib.licenses.mit;
    mainProgram = "emcsync";
    platforms = lib.attrNames sources;
  };
}
