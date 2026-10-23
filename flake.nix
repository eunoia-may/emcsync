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
      # 暂不提供 packages：尚无 Cargo.toml / Cargo.lock，buildRustPackage 会失败。
      # cargo 落地后再补 packages.default，用 cargoLock.lockFile = ./Cargo.lock。
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
