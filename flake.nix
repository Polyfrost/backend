{
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay.url = "github:oxalica/rust-overlay";
    };

    outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
        flake-utils.lib.eachDefaultSystem (system: let pkgs = nixpkgs.legacyPackages.${system}; in {
            devShells.default = pkgs.mkShell {
                packages = let
                    rust-bin = rust-overlay.lib.mkRustBin {} pkgs;
                    rust' = (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
                in [
                    rust'
                ] ++ (with pkgs; [
                    pkg-config
                    openssl.dev
                ]);
            };
        });
}
