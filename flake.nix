{
  description = "Custom anyrun plugins";
  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";
  outputs = { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "aarch64-linux" "x86_64-linux" ];
      
      makePackage = pkgs: pkgs.rustPlatform.buildRustPackage {
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
      };
    in
    {
      packages = forAllSystems (system: {
        default = makePackage nixpkgs.legacyPackages.${system};
      });

      devShells = forAllSystems (system: {
        default = nixpkgs.legacyPackages.${system}.callPackage ./shell.nix {
          default-package = makePackage nixpkgs.legacyPackages.${system};
        };
      });
    };
}