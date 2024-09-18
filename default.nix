let sources = import ./npins; in
{ pkgs ? import sources.nixpkgs { config = {}; overlays = []; } }:
let
  examplesDir = ./examples;

  inherit (pkgs.lib.attrsets) filterAttrs mapAttrs attrValues;
  inherit (pkgs.lib) path;

  mkExample = pkgs.callPackage ./mk-example.nix {};
  examples = mapAttrs
    (pname: _: mkExample { inherit pname; src = (path.append examplesDir pname); })
    (filterAttrs (n: v: v == "directory") (builtins.readDir examplesDir));
  exampleDrvs = attrValues examples;

  mkTest = pkgs.callPackage (import ./mk-test.nix) {};
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

  shell = pkgs.mkShell {
    inputsFrom = exampleDrvs;
    packages = with pkgs; [ rustfmt clippy ];
  };
} // examples
