{ lib, workspace }:
pname:
workspace.overrideAttrs {
  inherit pname;
  # HACK: think i need version override here because buildRustPackage checks that it is the same as in Cargo.toml
  # probably a way to disable this, though i can't be bothered rn
  version = "0.1.0";

  # examples are actually individual sub-crates (because theyre in a workspace?)
  # cargoBuildFlags = "--example ${pname}";
  cargoBuildFlags = "--package ${pname}";

  fixupPhase = ''
    patchelf --set-rpath "${lib.makeLibraryPath workspace.rpathLibs}" "$out/bin/${pname}";
  '';
}
