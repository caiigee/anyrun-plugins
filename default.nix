{ pkgs, ... }:

with pkgs; rustPlatform.buildRustPackage {
  pname = "anyrun-plugins";
  version = "0.1.0";
  src = ./.;
  CARGO_BUILD_OPTS = "--incremental";
  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "anyrun-interface-0.1.0" = "sha256-fQ4LkmZeW4eGowbVfvct1hLFD0hNkZiX5SzRlWqhgxc=";
    };
  };
  buildInputs = [
    sqlite
    xdg-utils
  ];
}