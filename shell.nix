{ pkgs }:

let
  default-package = pkgs.callPackage ./default.nix { };
in
pkgs.mkShell {
  inputsFrom = [ default-package ];
  shellHook = ''
    echo "Successfully initialized anyrun-plugins development shell!"
  '';
}
