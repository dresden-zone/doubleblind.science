{
  inputs = { 
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05"; 
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
    };
  };

  outputs = { self, nixpkgs, naersk, fenix }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      makeTest = pkgs.callPackage "${nixpkgs}/nixos/tests/make-test-python.nix";

      toolchain = with fenix.packages.${system}; combine [
        latest.cargo
        latest.rustc
      ];

      migration-package = pkgs.callPackage ./derivation.nix {
        buildPackage = (naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        }).buildPackage;
      };
    in
    {
      checks.${system}.test-sea-orm-cli-migration =
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
                  ${migration-package}/bin/migration refresh --database-url postgresql://${username}:${password}@localhost/${database}
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

      packages.${system} = {
        update-schema = pkgs.writeScriptBin "update-schema" ''
          nix build ${self}#checks.${system}.test-sea-orm-cli-migration
          BUILD_DIR=$(nix build ${self}#checks.${system}.test-sea-orm-cli-migration --no-link --print-out-paths)
          rm -rf entity/doubleblind.science/models/*
          cp -r $BUILD_DIR/out/* ./entity/doubleblind.science/models/
          #mv ./entity/doubleblind.science/mod.rs ./entity/doubleblind.science/lib.rs
          chmod -R 644 ./entity/doubleblind.science/models/*
          ${pkgs.cargo}/bin/cargo fmt
        '';

        run-migration-based = pkgs.writeScriptBin "run-migration" ''
          ${pkgs.sea-orm-cli}/bin/sea-orm-cli migration run --migration-dir ${self}/migrations-based
        '';
      };

      devShells."x86_64-linux".default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [ pkg-config postgresql_14 openssl sea-orm-cli ];
      };
    };
}
