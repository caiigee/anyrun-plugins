{
  description = "Custom Anyrun plugins";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
        
        buildPlugin = path: pkgs.rustPlatform.buildRustPackage {
          pname = "anyrun-${path}";
          version = "0.1.0";
          src = ./${path};
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          buildInputs = with pkgs; [ ];
          nativeBuildInputs = with pkgs; [ rustVersion ];
        };
      in
      {
        packages = {
          # Standalone:
          applications = buildPlugin "applications";
          bookmarks = buildPlugin "Browser/bookmarks";
          webapps = buildPlugin "Browser/webapps";
          webpages = buildPlugin "Browser/webpages";
          websearch = buildPlugin "Browser/websearch";
          powermenu = buildPlugin "powermenu";
          rink = buildPlugin "rink";
          shell = buildPlugin "shell";
          
          # All-in-one:
          default = pkgs.symlinkJoin {
            name = "all-plugins";
            paths = [
              self.packages.${system}.applications
              self.packages.${system}.bookmarks
              self.packages.${system}.webapps
              self.packages.${system}.webpages
              self.packages.${system}.websearch
              self.packages.${system}.powermenu
              self.packages.${system}.rink
              self.packages.${system}.shell
            ];
          };
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustVersion
            rust-analyzer
            clippy
          ];
        };
      }
    );
}