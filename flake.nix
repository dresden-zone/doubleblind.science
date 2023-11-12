{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    utils = {
      url = "github:numtide/flake-utils";
    };

    fenix = {
      url = "github:nix-community/fenix";
    };
  };

  outputs = inputs@{ self, nixpkgs, naersk, utils, fenix, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          toolchain = with fenix.packages.${system}; combine [
            latest.cargo
            latest.rustc
          ];

          package = pkgs.callPackage ./derivation.nix {
            buildPackage = (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage;
          };

          test-vm-pkg = self.nixosConfigurations.doubleblind-mctest.config.system.build.vm;

        in
        rec {
          checks = packages;
          packages = {
            doubleblind = package;
            default = package;
          };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs = (with packages.default; nativeBuildInputs ++ buildInputs) ++ [
              # python for running test scripts
              (pkgs.python3.withPackages (p: with p; [
                requests
              ]))
            ];
          };
        }
      ) // {
      overlays.default = final: prev: {
        inherit (self.packages.${prev.system})
          doubleblind;
      };

      nixosModules = rec {
        doubleblind = import ./nixos-module/doubleblind.nix;
        default = doubleblind;
      };

      hydraJobs =
        let
          hydraSystems = [
            "x86_64-linux"
            "aarch64-linux"
          ];
        in
        builtins.foldl'
          (hydraJobs: system:
            builtins.foldl'
              (hydraJobs: pkgName:
                nixpkgs.lib.recursiveUpdate hydraJobs {
                  ${pkgName}.${system} = self.packages.${system}.${pkgName};
                }
              )
              hydraJobs
              (builtins.attrNames self.packages.${system})
          )
          { }
          hydraSystems;
    };
}
