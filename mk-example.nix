{
  lib,
  rustPlatform,
  expat,
  freetype,
  libX11,
  libxcb,
  libXcursor,
  libXi,
  libxkbcommon,
  libXrandr,
  vulkan-loader,
  wayland,
}:
pname:
let
  rpathLibs = [
    libXcursor
    libXi
    libxkbcommon
    libXrandr
    libX11
    vulkan-loader
    wayland
  ];
in
  rustPlatform.buildRustPackage {
    inherit pname;
    version = "0.1.0";

    src = let
      fs = lib.fileset;
      fileset = fs.difference
	(fs.gitTracked ./.)
	(fs.unions [
	  ./npins
	  (fs.fileFilter (f: f.hasExt "nix") ./.)
	]);
    in
      fs.toSource {
        root = ./.;
        inherit fileset;
      };

    # examples are actually individual sub-crates (because theyre in a workspace?)
    # cargoBuildFlags = "--example ${pname}";
    cargoBuildFlags = "--package ${pname}";
    
    cargoLock.lockFile = ./Cargo.lock;
    cargoLock.outputHashes = {
      "dpi-0.1.1" = "sha256-25sOvEBhlIaekTeWvy3UhjPI1xrJbOQvw/OkTg12kQY=";
      "glyphon-0.5.0" = "sha256-OGXLqiMjaZ7gR5ANkuCgkfn/I7c/4h9SRE6MZZMW3m4=";
    };

    buildInputs = [
      expat
      freetype
      libxcb
      libX11
      libxkbcommon
    ];

    fixupPhase = ''
      patchelf --0.5.-set-rpath "${lib.makeLibraryPath rpathLibs}" "$out/bin/${pname}";
    '';

    passthru = { inherit rpathLibs; };
  }
