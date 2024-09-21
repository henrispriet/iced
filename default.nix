let sources = import ./npins; in
{ pkgs ? import sources.nixpkgs { config = {}; overlays = []; } }:
let
  examplesDir = ./examples;

  inherit (pkgs) lib;

  mkExample = pkgs.callPackage ./mk-example.nix { inherit workspace; };
  mkTest = pkgs.callPackage ./mk-test.nix {};

  examples = lib.mapAttrs
    # (lib.flip (lib.const mkExample))
    (n: _: mkExample n)
    (lib.filterAttrs
      (n: v: v == "directory")
      (builtins.readDir examplesDir)
    );
  exampleDrvs = lib.attrValues examples;

  # build the workspace
  workspace = pkgs.callPackage ./workspace.nix {};
in {
  inherit workspace;

  # build all examples (individually, does not use workspace)
  # recommended to build with --keep-going, to keep building other examples when one fails
  # example use: `nix-build -A all --keep-going`
  all = pkgs.symlinkJoin {
    name = "all-examples";
    paths = exampleDrvs;
  };

  # test all examples
  # builds the whole workspace to test, to build all examples individually, use `nix-build -A all`
  # recommended to build with --keep-going, to keep running other tests when one fails
  # example use: `nix-build -A test-all --keep-going`
  test-all = pkgs.symlinkJoin {
    name = "all-tests";
    paths = builtins.map
      (args: mkTest (args // { pkg = workspace; }))
      (lib.cartesianProduct {
	displayServer = [ "x11" "wayland" ];
	exe = lib.attrNames examples;
      });
  };

  # test one example
  # example use: `nix-build -A test --argstr displayServer wayland --argstr example arc`
  # example use: `nix-build -A test.driverInteractive --argstr displayServer wayland --argstr example arc`
  test = { displayServer, example }: mkTest {
    inherit displayServer;
    pkg = examples.${example};
    exe = example;
  };

  shell = pkgs.mkShell {
    inputsFrom = [ workspace ];
    packages = with pkgs; [ rustfmt clippy ];
  };
} // examples
