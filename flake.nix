{
  description = "eunoia-mc-content-sync 开发环境";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      forEachSystem = f:
        nixpkgs.lib.genAttrs systems (system:
          f (import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          }));
    in
    {
      packages = forEachSystem (pkgs:
        let
          rust = pkgs.rust-bin.stable.latest.default;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "emcsync-cli";
            version = (pkgs.lib.importTOML ./Cargo.toml).package.version;

            # 只收编译需要的文件；target、本地 cargo 缓存和测试实例都不进 store。
            src = pkgs.lib.fileset.toSource {
              root = ./.;
              fileset = pkgs.lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./src
                ./locales
              ];
            };

            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "复现 Minecraft 实例的内容资源";
              mainProgram = "emcsync-cli";
            };
          };
        });

      devShells = forEachSystem (pkgs:
        let
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [ rust ];

            RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";
          };
        });
    };
}
