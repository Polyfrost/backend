{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
        crane.url = "github:ipetkov/crane";
    };

    outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, ... }:
        flake-utils.lib.eachDefaultSystem (system: let
            # Initialize nixpkgs
            pkgs = nixpkgs.legacyPackages.${system};
            # Setup the rust toolchain
            rust-bin = rust-overlay.lib.mkRustBin {} pkgs;
            rust' = (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
            # Setup rust nix packaging
            craneLib = (crane.mkLib pkgs).overrideToolchain (_: rust');
            commonArgs = {
                src = craneLib.cleanCargoSource ./.;
                strictDeps = true;

                buildInputs = with pkgs; [
                    openssl
                ];
                nativeBuildInputs = with pkgs; [
                    pkg-config
                ];
            };
            cranePackage = craneLib.buildPackage (commonArgs // {
                cargoArtifacts = craneLib.buildDepsOnly commonArgs;
            });
        in {
            packages = {
                default = self.packages.${system}.backend;
                backend = cranePackage;
            };
            devShells.default = craneLib.devShell {
                packages = cranePackage.nativeBuildInputs; # Add all build-time dependencies to the environment
            };
        });
}
