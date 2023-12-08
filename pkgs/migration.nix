{ buildPackage, lib, pkg-config, openssl }:
buildPackage {
  pname = "doubleblind-migration";
  version = "0.1.0";

  src = ../.;
  cargoBuildOptions = (x: x ++ [ "-p migration" ]);
  cargoSha256 = lib.fakeSha256;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ ];

  meta = {
    description = "";
    homepage = "https://github.com/dresden-zone/doubleblind.science";
  };
}
