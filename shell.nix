{ pkgs }:

let
  default-package = pkgs.callPackage ./default.nix;
  # TESTING
  # Writing the config file:
  mkAnyrunTestConfig = pkgs.writeText "config.ron" ''
    Config(
      x: Fraction(0.5),
      y: Fraction(0.33),
      width: Absolute(500),
      height: Absolute(100),
      plugins: [
        "${toString default-package}/lib/libapplications.so",
        "${toString default-package}/lib/libbookmarks.so",
        "${toString default-package}/lib/libwebapps.so",
        "${toString default-package}/lib/libwebpages.so",
        "${toString default-package}/lib/libwebsearch.so",
        "${toString default-package}/lib/libpowermenu.so",
        "${toString default-package}/lib/librink.so",
        "${toString default-package}/lib/libshell.so"
      ]
    )
  '';
  
  # Making a directory with the config file:
  anyrunConfigDir = pkgs.runCommand "anyrun-config" {} ''
    mkdir -p $out
    cp ${mkAnyrunTestConfig} $out/config.ron
  '';

  # Defining the script to run anyrun with the test configuration:
  anyrunTestScript = pkgs.writeShellScriptBin "test-anyrun" ''
    ${pkgs.anyrun}/bin/anyrun -c ${anyrunConfigDir}
  '';
  
  # DEVELOPMENT
  rust-tools = with pkgs; [
    clippy
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
  inputsFrom = [ default-package ];
  buildInputs = with pkgs; [
    rust-tools
    anyrunTestScript
    anyrun
  ];
  shellHook = ''
    ${mkToolLinks rust-tools}
    echo "Successfully initialized anyrun-plugins development shell!"
  '';
}
