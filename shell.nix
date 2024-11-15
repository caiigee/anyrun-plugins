{ pkgs }:

let
  default-package = pkgs.callPackage ./default.nix { };
  rust-tools = with pkgs; [
    clippy
    rust-analyzer
    rustfmt
  ];
  mkToolLinks = tools: ''
    mkdir -p .env/bin
    ${builtins.concatStringsSep "\n" (map (tool: ''
      for toolpath in ${tool}/bin/*; do
        ln -sf $toolpath .env/bin/$(basename $toolpath)
      done
    '') tools)}
  '';
in
pkgs.mkShell {
  # For some reason rust-analyzer doesn't work without this.
  inputsFrom = [ default-package ];
  buildInputs = [ 
    rust-tools
  ];
  shellHook = ''
    ${mkToolLinks rust-tools}
    echo "Successfully initialized anyrun-plugins development shell!"
  '';
}
