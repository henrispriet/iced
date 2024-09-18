let sources = import ./npins; in
{ pkgs ? import sources.nixpkgs { config = {}; overlays = []; } }:
let
  examplesDir = ./examples;

  inherit (pkgs.lib.attrsets) filterAttrs mapAttrs attrValues;
  inherit (pkgs.lib) path;

  mkExample = pkgs.callPackage ./mk-example.nix {};
  mkTest = pkgs.callPackage (import ./mk-test.nix) {};

  examples = mapAttrs
    (pname: _: mkExample { inherit pname; src = (path.append examplesDir pname); })
    (filterAttrs (n: v: v == "directory") (builtins.readDir examplesDir));
  exampleDrvs = attrValues examples;

in {
  # build all examples
  all = pkgs.buildEnv {
    name = "all-examples";
    paths = exampleDrvs;
  };

  # test all examples
  test-all = pkgs.buildEnv {
    name = "all-tests";
    paths = (builtins.map (mkTest "wayland") exampleDrvs)
         ++ (builtins.map (mkTest "x11") exampleDrvs);
  };

  # test one example
  # example use: `nix-build -A test --argstr displayServer wayland --argstr example arc`
  # example use: `nix-build -A test.driverInteractive --argstr displayServer wayland --argstr example arc`
  test = { displayServer, example }: mkTest displayServer examples.${example};

  shell = pkgs.mkShell {
    inputsFrom = exampleDrvs;
    packages = with pkgs; [ rustfmt clippy ];
  };
} // examples
