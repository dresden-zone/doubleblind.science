{  buildPackage, lib, pkg-config, openssl, boringssl }:
buildPackage {
  pname = "doubleblind-backend";
  version = "0.1.0";

  src = ../.;

  cargoSha256 = lib.fakeSha256;

  buildInputs = [ pkg-config openssl ];

  meta = {
    description = "service which promotes open science and helps with publishing artifacts";
    homepage = "https://github.com/dresden-zone/doubeblind.science";
  };
}
