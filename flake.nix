{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        packages = with pkgs; [
          (rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          })
          pkg-config
          openssl
          sqlx-cli
          postgresql
          cargo-audit
          cargo-watch
          # Keep in sync with the go directive in go/go.mod and the golang
          # image pins in Dockerfile, Dockerfile.arm and .gitlab-ci.yml.
          go_1_27
          gotools
          gopls
          golint
          pre-commit
          hadolint
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = packages;
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath packages}";
          RUST_LOG = "debug";
        };
      }
    );
}
