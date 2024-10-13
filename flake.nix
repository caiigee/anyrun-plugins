{
  description = "Custom Anyrun plugins";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustVersion = pkgs.rust-bin.stable.latest.default;

        buildWorkspace = pkgs.rustPlatform.buildRustPackage {
          pname = "anyrun-plugins";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "anyrun-interface-0.1.0" = "sha256-fQ4LkmZeW4eGowbVfvct1hLFD0hNkZiX5SzRlWqhgxc=";
            };
          };
          buildInputs = with pkgs; [ sqlite ];
          nativeBuildInputs = with pkgs; [ rustVersion ];
        };

        pluginNames = [
          "applications"
          "bookmarks"
          "webapps"
          "webpages"
          "websearch"
          "powermenu"
          "rink"
          "shell"
        ];
      in
      {
        packages = {
          default = buildWorkspace;
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustVersion
            rust-analyzer
            clippy
            sqlite
          ];
        };
      }
    );
}
