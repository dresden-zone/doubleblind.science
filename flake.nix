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
          makeTest = pkgs.callPackage "${nixpkgs}/nixos/tests/make-test-python.nix";

          toolchain = with fenix.packages.${system}; combine [
            stable.cargo
            stable.rustc
          ];

          migration = pkgs.callPackage ./pkgs/migration.nix {
            buildPackage = (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage;
          };

          backend = pkgs.callPackage ./pkgs/backend.nix {
            buildPackage = (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage;
          };

          frontend = pkgs.callPackage ./pkgs/frontend.nix {
            mkPnpmPackage = pnpm2nix.packages."${system}".mkPnpmPackage;
            domain = "science.tanneberger.me";
          };
        in
        rec {
          checks.test-sea-orm-cli-migration =
              let
                username = "postgres";
                password = "password";
                database = "database";
                migrations_dir = ./migration;
              in
              makeTest
                {
                  name = "test-sea-orm-cli-migration";
                  nodes = {
                    server = { lib, config, pkgs, ... }: {
                      services.postgresql = {
                        enable = true;
                        ensureDatabases = [ database ];
                        ensureUsers = [{
                          name = username;
                          ensurePermissions = {
                            "DATABASE ${database}" = "ALL PRIVILEGES";
                          };
                        }];
                        initialScript = pkgs.writeScript "initScript" ''
                          ALTER USER postgres WITH PASSWORD '${password}';
                        '';
                      };

                      systemd.services.postgresql.postStart = lib.mkAfter ''
                        ${migration}/bin/migration refresh --database-url postgresql://${username}:${password}@localhost/${database}
                      '';
                    };
                  };
                  testScript = ''
                    start_all()
                    server.wait_for_unit("postgresql.service")
                    server.execute("${pkgs.sea-orm-cli}/bin/sea-orm-cli generate entity --database-url postgresql://${username}:${password}@localhost/${database} --date-time-crate time --with-serde both --output-dir /tmp/out")
                    server.copy_from_vm("/tmp/out", "")
                  '';
                }
                {
                  inherit pkgs;
                  inherit (pkgs) system;
                };

          packages = {
            update-schema = pkgs.writeScriptBin "update-schema" ''
              nix build ${self}#checks.${system}.test-sea-orm-cli-migration
              BUILD_DIR=$(nix build ${self}#checks.${system}.test-sea-orm-cli-migration --no-link --print-out-paths)
              rm -rf entity/src/models/*
              cp -r $BUILD_DIR/out/* ./entity/src/models/
              chmod -R 644 ./entity/src/models/*
              ${pkgs.cargo}/bin/cargo fmt
            '';

            run-migration-based = pkgs.writeScriptBin "run-migration" ''
              ${pkgs.sea-orm-cli}/bin/sea-orm-cli migration run --migration-dir ${self}/migrations-based
            '';
            inherit backend frontend;

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
          doubleblind-backend doubleblind-frontend;
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
