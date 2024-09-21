let
  sources = import ./npins;
  # HACK: common is not exposed via an attribute, so get it by path instead
  common = "${sources.nixpkgs}/nixos/tests/common";
in
{ lib, nixosTest }:
# adapted from https://github.com/NixOS/nixpkgs/blob/d0b8a0b81552dd2647de199a3d11775d25007bec/nixos/tests/freetube.nix
# tysm to kirillrdy tbh!!!
let
  inherit (lib) getExe;

  tests = {
    wayland = package: {...}: {
      imports = [ "${common}/wayland-cage.nix" ];
      services.cage.program = getExe package;
      virtualisation.memorySize = 2047;
      environment.variables.NIXOS_OZONE_WL = "1";
      environment.variables.DISPLAY = "do not use";
    };
    x11 = package: {...}: {
      imports = [ "${common}/user-account.nix" "${common}/x11.nix" ];
      virtualisation.memorySize = 2047;
      services.xserver.enable = true;
      services.xserver.displayManager.sessionCommands = getExe package;
      test-support.displayManager.auto.user = "alice";
    };
  };
in
  displayServer: package:
    nixosTest ({...}: {
      name = "test-${package.name}-${displayServer}";
      nodes = { machine = tests.${displayServer} package; };
      # time-out on ofborg
      # meta.broken = pkgs.stdenv.isAarch64;
      enableOCR = true;

      testScript = ''
        start_all()
        machine.wait_for_unit('graphical.target')
        machine.sleep(3)
        machine.screenshot("${package.name}-${displayServer}-screen")
      '';
    })
