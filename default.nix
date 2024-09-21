let sources = import ./npins; in
{ pkgs ? import sources.nixpkgs { config = {}; overlays = []; } }:
let
  examplesDir = ./examples;

  inherit (pkgs) lib;

  mkExample = pkgs.callPackage ./mk-example.nix {};
  mkTest = pkgs.callPackage ./mk-test.nix {};

  examples = lib.mapAttrs
    # (lib.flip (lib.const mkExample))
    (n: _: mkExample n)
    (lib.filterAttrs
      (n: v: v == "directory")
      (builtins.readDir examplesDir)
    );
  exampleDrvs = lib.attrValues examples;

in {
  # build all examples
  # recommended to build with --keep-going, to keep building other examples when one fails
  # example use: `nix-build -A all --keep-going`
  all = pkgs.symlinkJoin {
    name = "all-examples";
    paths = exampleDrvs;
  };

  # test all examples
  # recommended to build with --keep-going, to keep running other tests when one fails
  # example use: `nix-build -A test-all --keep-going`
  test-all = pkgs.symlinkJoin {
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

    # FIXME: inputsFrom workspace instead?
    # shellHook = ''echo "evaluating depencies for all examples, this could take a while..."'';
  };
} // examples
