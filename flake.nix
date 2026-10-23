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
          default = pkgs.callPackage ./package.nix { inherit rustPlatform; };
        });

      # 只把包挂进 pkgs，不往 systemPackages 或 home.packages 里塞东西，装不装由用户决定。
      # 必须转发到 self.packages.${system}.default：后者用 flake.lock 钉住的 nixpkgs，
      # 缓存能命中；若改用 callPackage 重建，则跟随用户自己的 nixpkgs，缓存永不命中。
      overlays.default = final: prev: {
        emcsync-cli = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };

      # NixOS 用户在 modules 列表里引用。
      # mkAfter 让本 overlay 排后，用户自己的 overlay 可覆盖同名包。
      nixosModules.default = { lib, ... }: {
        nixpkgs.overlays = lib.mkAfter [ self.overlays.default ];
      };

      # home-manager 用户在 imports 列表里引用。
      homeModules.default = { lib, ... }: {
        nixpkgs.overlays = lib.mkAfter [ self.overlays.default ];
      };

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
