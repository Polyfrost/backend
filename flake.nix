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
                commonArgs = {
                    src = craneLib.cleanCargoSource ./.;
                    strictDeps = true;

                    buildInputs = with pkgs; [ openssl ];
                    nativeBuildInputs = with pkgs; [ pkg-config ];
                };
                cranePackage = craneLib.buildPackage (
                    commonArgs
                    // {
                        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
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
                checks.formatting = treefmtEval.config.build.check self;
                devShells.default = craneLib.devShell {
                    # Add all build-time dependencies to the environment
                    packages =
                        cranePackage.buildInputs
                        ++ cranePackage.nativeBuildInputs
                        ++ (with pkgs; [
                            cargo-deny
                            evcxr
                            lldb
                            self.formatter.${system}
                        ]);
                };
            }
        );
}
