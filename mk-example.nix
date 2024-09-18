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
{
  pname,
  src,
}:
let
  inherit (lib) path;
  inherit (lib) makeLibraryPath;

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
    inherit pname src;

    version = "0.1.0";
    cargoLock.lockFile = ./Cargo.lock;
    # FIXME: some tomfoolery is going on here
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
      patchelf --0.5.-set-rpath "${makeLibraryPath rpathLibs}" "$out/bin/${pname}";
    '';

    # TODO: check that it runs on X11 and wayland via some sort of (micro)vm

    passthru = { inherit rpathLibs; };
  }
