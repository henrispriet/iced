{
  lib,
  rustPlatform,
  pkg-config,
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
    pname = "iced-workspace";
    version = "0.13.1";

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
    
    cargoLock.lockFile = ./Cargo.lock;

    nativeBuildInputs = [
      pkg-config
    ];

    buildInputs = [
      expat
      freetype
      libxcb
      libX11
      libxkbcommon
    ];

    # build all packages in workspace
    cargoBuildFlags = "--workspace";

    fixupPhase = ''
      for example in examples/*/; do
        exe=$(basename "$example")
        patchelf --set-rpath "${lib.makeLibraryPath rpathLibs}" "$out/bin/$exe";
      done
    '';

    passthru = { inherit rpathLibs; };
  }
