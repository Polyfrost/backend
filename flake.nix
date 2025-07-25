{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
        crane.url = "github:ipetkov/crane";
        advisory-db = {
            url = "github:rustsec/advisory-db";
            flake = false;
        };
        treefmt-nix = {
            url = "github:numtide/treefmt-nix";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };

    outputs =
        {
            self,
            nixpkgs,
            flake-utils,
            rust-overlay,
            crane,
            advisory-db,
            treefmt-nix,
            ...
        }:
        flake-utils.lib.eachDefaultSystem (
            system:
            let
                # Initialize nixpkgs
                pkgs = nixpkgs.legacyPackages.${system};
                inherit (pkgs) lib;
                # Setup the rust toolchain
                rust-bin = rust-overlay.lib.mkRustBin { } pkgs;
                rust' = (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
                # Setup rust nix packaging
                craneLib = (crane.mkLib pkgs).overrideToolchain (_: rust');
                commonArgs = {
                    src = craneLib.cleanCargoSource ./.;
                    strictDeps = true;

                    buildInputs = with pkgs; [ openssl ];
                    nativeBuildInputs = with pkgs; [ pkg-config ];
                };
                cargoArtifacts = craneLib.buildDepsOnly commonArgs;
                commonArgsWithDeps = commonArgs // {
                    inherit cargoArtifacts;
                };
                cranePackage = craneLib.buildPackage (
                    commonArgsWithDeps
                    // {
                        meta = {
                            mainProgram = "backend";
                            license = lib.licenses.gpl3Plus;
                        };
                    }
                );
                # Setup treefmt-nix
                treefmtModule = import ./treefmt.nix { inherit rust'; };
                treefmtEval = treefmt-nix.lib.evalModule pkgs treefmtModule;
            in
            {
                packages = {
                    default = self.packages.${system}.backend;
                    backend = cranePackage;
                };
                formatter = treefmtEval.config.build.wrapper;
                checks = {
                    formatting = treefmtEval.config.build.check self;
                    clippy = craneLib.cargoClippy (
                        commonArgsWithDeps // { cargoClippyExtraArgs = "--all-targets -- --deny warnings"; }
                    );
                    deny =
                        let
                            git = ''HOME="$GIT_HOME" git'';
                            gitInit = ''
                                ${git} config --global init.defaultBranch "main"
                                ${git} config --global user.email "example@example.com"
                                ${git} config --global user.name "John Doe"
                                ${git} init
                                ${git} add -A
                                ${git} commit -m "init"
                            '';
                        in
                        craneLib.cargoDeny (
                            commonArgs
                            // {
                                cargoDenyChecks = "--disable-fetch all";
                                nativeBuildInputs = [ pkgs.git ];
                                configurePhase = ''
                                    runHook preConfigure

                                    DB_PATH="$CARGO_HOME"/advisory-dbs/advisory-db-3157b0e258782691
                                    mkdir -p "$DB_PATH"

                                    pushd "$DB_PATH"

                                    ln -s ${advisory-db}/{*,.*} .
                                    GIT_HOME="$(mktemp -d)"
                                    ${gitInit} # Cargo-deny complains if it isn't a real repo

                                    popd

                                    runHook postConfigure
                                '';
                            }
                        );
                    udeps = craneLib.mkCargoDerivation (
                        commonArgsWithDeps
                        // {
                            nativeBuildInputs = [ pkgs.cargo-udeps ];
                            pnameSuffix = "-udeps";
                            buildPhaseCargoCommand = ''
                                cargo --offline \
                                    udeps \
                                    --all-targets \
                                    --all-features
                            '';
                        }
                    );
                };
                devShells.default = craneLib.devShell {
                    # Add all build-time dependencies to the environment
                    packages =
                        cranePackage.buildInputs
                        ++ cranePackage.nativeBuildInputs
                        ++ (with pkgs; [
                            cargo-deny
                            cargo-udeps
                            evcxr
                            lldb
                            self.formatter.${system}
                        ]);
                };
            }
        );
}
