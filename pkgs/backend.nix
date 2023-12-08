{  buildPackage, lib, pkg-config, openssl }:
buildPackage {
  pname = "doubleblind-backend";
  version = "0.1.0";

  src = ../.;

  cargoSha256 = lib.fakeSha256;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];

  meta = {
    description = "service which promotes open science and helps with publishing artifacts";
    homepage = "https://github.com/dresden-zone/doubeblind.science";
  };
}
