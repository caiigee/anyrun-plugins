{ pkgs }:

let
  default-package = pkgs.callPackage ./default.nix { };
  # Writing the config file:
  mkAnyrunConfig = pkgs.writeText "config.ron" ''
    Config(
      x: Fraction(0.5),
      y: Fraction(0.33),
      width: Absolute(500),
      height: Absolute(100),
      plugins: [
        "${default-package}/lib/libapplications.so",
        "${default-package}/lib/libbookmarks.so",
        "${default-package}/lib/libwebpages.so",
        "${default-package}/lib/libpowermenu.so",
        "${default-package}/lib/librink.so",
        "${default-package}/lib/libshell.so",
        "${default-package}/lib/libwebsearch.so",
      ]
    )
  '';
  
  # Writing the config for websearch:
  mkBrowserConfig = pkgs.writeText "browser.ron" ''
    Config(
      command_prefix: Some("uwsm app -- ")
    )
  '';

  # # Making a directory with the config file:
  # anyrunConfigDir = pkgs.runCommand "anyrun-config" {} ''

  # '';

  # Defining the script to run anyrun with the test configuration:
  anyrunTestScript = pkgs.writeShellScriptBin "test-anyrun" ''
    ${pkgs.anyrun}/bin/anyrun -c
    mkdir -p $out
    cp ${mkAnyrunConfig} $out/config.ron
    cp ${mkBrowserConfig} $out/browser.ron
  '';
in
  anyrunTestScript
