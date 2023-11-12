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

    pnpm2nix = {
      url = "github:nzbr/pnpm2nix-nzbr";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
  };

  outputs = inputs@{ self, nixpkgs, naersk, utils, fenix, pnpm2nix, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          toolchain = with fenix.packages.${system}; combine [
            latest.cargo
            latest.rustc
          ];

          backend = pkgs.callPackage ./backend.nix {
            buildPackage = (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage;
          };

          frontend = pkgs.callPackage ./frontend.nix {
            mkPnpmPackage = pnpm2nix.packages."${system}".mkPnpmPackage;
            domain = "science.tanneberger.me";
          };
        in
        rec {
          checks = packages;
          packages = {
            doubleblind-backend = backend;
            doubleblind-frontend = frontend;

            default = backend;
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
