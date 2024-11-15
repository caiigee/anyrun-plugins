{
  description = "Custom anyrun plugins";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { nixpkgs, rust-overlay, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "aarch64-linux" "x86_64-linux" ];
      pkgsForSystem = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
    in
    {
      packages = forAllSystems (system: {
        default = (pkgsForSystem system).callPackage ./default.nix { };
      });

      devShells = forAllSystems (system: {
        default = (pkgsForSystem system).callPackage ./shell.nix { };
      });

      test = forAllSystems (system: {
        default = (pkgsForSystem system).callPackage ./test.nix { };
      });
    };
}
