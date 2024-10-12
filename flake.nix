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
          };
          buildInputs = with pkgs; [ ];
          nativeBuildInputs = with pkgs; [ rustVersion ];
        };

        buildPlugin = name: pkgs.rustPlatform.buildRustPackage {
          pname = "anyrun-${name}";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          buildInputs = with pkgs; [ ];
          nativeBuildInputs = with pkgs; [ rustVersion ];
          cargoBuildFlags = [ "--package" name ];
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
        } // builtins.listToAttrs (map (name: { 
          inherit name; 
          value = buildPlugin name;
        }) pluginNames);

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