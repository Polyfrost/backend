{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
        crane.url = "github:ipetkov/crane";
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
                stdenvSelector = p: if p.stdenv.hostPlatform.isElf then p.stdenvAdapters.useMoldLinker p.stdenv else p.stdenv;
                commonArgs = {
                    src = craneLib.cleanCargoSource ./.;
                    strictDeps = true;

                    buildInputs = with pkgs; [ openssl ];
                    nativeBuildInputs = with pkgs; [ pkg-config ];

                    # Use mold linker for faster builds on ELF platforms
                    stdenv = stdenvSelector;
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
                    deny = craneLib.cargoDeny (
                        commonArgs
                        // {
                            cargoDenyChecks = "all";

                            # Used to allow network access so yanked crates and advisories can be downloaded
                            outputHash = "sha256-pQpattmS9VmO3ZIQUFn66az8GSmB4IvYhTTCFn6SUmo=";
                            outputHashAlgo = "sha256";
                            outputHashMode = "recursive";
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
                devShells.default = craneLib.devShell.override {
                    mkShell = pkgs.mkShell.override {
                        stdenv = stdenvSelector pkgs;
                    };
                } {
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
