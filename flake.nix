{
  description = "Kally package manager";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    in { packages = nixpkgs.lib.genAttrs systems (system:
      let pkgs = import nixpkgs { inherit system; };
      in { kally = pkgs.rustPlatform.buildRustPackage {
        pname = "kally"; version = "0.1.0"; src = ./.;
        cargoLock = { lockFile = ./Cargo.lock; allowBuiltinFetchGit = true; };
      }; default = self.packages.${system}.kally; }); };
}
