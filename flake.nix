{
  description = "Anyrun plugins";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    systems.url = "github:nix-systems/default-linux";
    flake-utils.url = "github:numtide/flake-utils";
    flake-utils.inputs.systems.follows = "systems";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    advisory-db,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      inherit (pkgs) lib;

      craneLib = crane.mkLib pkgs;
      src = craneLib.cleanCargoSource ./.;

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = true;

        buildInputs = [
          pkgs.sqlite
          pkgs.xdg-utils
        ];

        # Additional environment variables can be set directly
        # MY_CUSTOM_VAR = "some value";
      };

      craneLibLLvmTools =
        craneLib.overrideToolchain
        (fenix.packages.${system}.complete.withComponents [
          "cargo"
          "llvm-tools"
          "rustc"
        ]);

      # Build *just* the cargo dependencies (of the entire workspace),
      # so we can reuse all of that work (e.g. via cachix) when running in CI
      # It is *highly* recommended to use something like cargo-hakari to avoid
      # cache misses when building individual top-level-crates
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      individualCrateArgs =
        commonArgs
        // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

      fileSetForCrate = crate:
        lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            (craneLib.fileset.commonCargoSources ./crates/common)
            (craneLib.fileset.commonCargoSources ./crates/workspace-hack)
            (craneLib.fileset.commonCargoSources crate)
          ];
        };

      # Build the top-level crates of the workspace as individual derivations.
      # This allows consumers to only depend on (and build) only what they need.
      # Though it is possible to build the entire workspace as a single derivation,
      # so this is left up to you on how to organize things
      #
      # Note that the cargo workspace must define `workspace.members` using wildcards,
      # otherwise, omitting a crate (like we do below) will result in errors since
      # cargo won't be able to find the sources for all members.
      # Define a function that creates a derivation for a given crate
      makeCrate = crateName:
        craneLib.buildPackage (individualCrateArgs
          // {
            pname = crateName;
            cargoExtraArgs = "--lib";
            src = fileSetForCrate ./crates + "${crateName}";
          });

      # Get a list of all crate directories in the `crates` folder
      crateNames = builtins.attrNames (builtins.readDir ./crates);

      # Dynamically generate a set of derivations for all crates
      plugins = builtins.listToAttrs (map (crateName: {
          name = crateName;
          value = makeCrate crateName;
        })
        crateNames);

      # TESTING
      mkAnyrunConfig = pkgs.writeText "config.ron" ''
        Config(
          x: Fraction(0.5),
          y: Fraction(0.33),
          width: Absolute(500),
          height: Absolute(100),
          plugins: [
            "${plugins.applications}/lib/libapplications.so",
          ]
        )
      '';

      mkBrowserConfig = pkgs.writeText "browser.ron" ''
        BrowserConfig(
          command_prefix: Some("uwsm app -- ")
        )
      '';

      anyrunConfigDir = pkgs.runCommand "anyrun-config" {} ''
        mkdir -p $out
        cp ${mkAnyrunConfig} $out/config.ron
        cp ${mkBrowserConfig} $out/browser.ron
      '';
    in {
      checks = {
        # Build the crates as part of `nix flake check` for convenience
        inherit plugins;

        # Run clippy (and deny all warnings) on the workspace source,
        # again, reusing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        my-workspace-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        my-workspace-doc = craneLib.cargoDoc (commonArgs
          // {
            inherit cargoArtifacts;
          });

        # Check formatting
        my-workspace-fmt = craneLib.cargoFmt {
          inherit src;
        };

        my-workspace-toml-fmt = craneLib.taploFmt {
          src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
          # taplo arguments can be further customized below as needed
          # taploExtraArgs = "--config ./taplo.toml";
        };

        # Audit dependencies
        my-workspace-audit = craneLib.cargoAudit {
          inherit src advisory-db;
        };

        # Audit licenses
        my-workspace-deny = craneLib.cargoDeny {
          inherit src;
        };

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on other crate derivations
        # if you do not want the tests to run twice
        my-workspace-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

        # Ensure that cargo-hakari is up to date
        my-workspace-hakari = craneLib.mkCargoDerivation {
          inherit src;
          pname = "my-workspace-hakari";
          cargoArtifacts = null;
          doInstallCargoArtifacts = false;

          buildPhaseCargoCommand = ''
            cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
            cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
            cargo hakari verify
          '';

          nativeBuildInputs = [
            pkgs.cargo-hakari
          ];
        };
      };

      packages = {
        inherit (plugins);
        my-workspace-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs
          // {
            inherit cargoArtifacts;
          });
        test = pkgs.writeShellScriptBin "test-anyrun" ''
          ${pkgs.anyrun}/bin/anyrun -c ${anyrunConfigDir}
        '';
      };

      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [
          pkgs.cargo-hakari
          pkgs.rust-analyzer
        ];

        shellHook = ''
          echo "Successfully initialized development shell!"
        '';
      };
    });
}
