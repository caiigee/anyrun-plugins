{ pkgs, ... }:

pkgs.rustPlatform.buildRustPackage {
  pname = "anyrun-plugins";
  version = "0.1.0";
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "anyrun-interface-0.1.0" = "sha256-fQ4LkmZeW4eGowbVfvct1hLFD0hNkZiX5SzRlWqhgxc=";
    };
  };
  buildInputs = with pkgs; [
    sqlite
    xdg-utils
    ps
    lsof
  ];
}

