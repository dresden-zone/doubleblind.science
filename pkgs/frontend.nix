{ domain, mkPnpmPackage }:
mkPnpmPackage {
    src = ../doubleblind-frontend/.;

    installInPlace = true;

    #postPatch = ''
    #  substituteInPlace src/app/data/api.domain.ts \
    #    --replace 'staging.tlm.solutions' '${domain}'
    #'';

    script = "build";

    installPhase = ''
      mkdir -p $out/bin
      cp -r ./dist/doubleblind-frontend/* $out/bin/
    '';
}
