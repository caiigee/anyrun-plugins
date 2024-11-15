{ pkgs }:

let
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
  # inputsFrom = [ default-package ];
  buildInputs = [
    rust-tools
    # anyrunTestScript
    pkgs.anyrun
  ];
  shellHook = ''
    ${mkToolLinks rust-tools}
    echo "Successfully initialized anyrun-plugins development shell!"
  '';
}
