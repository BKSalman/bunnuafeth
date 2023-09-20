{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.xserver.windowManager.bunnuafeth;
in
{
  ###### interface
  options = {
    services.xserver.windowManager.bunnuafeth.enable = mkEnableOption (lib.mdDoc "bunnuafeth");
  };

  ###### implementation
  config = mkIf cfg.enable {
    services.xserver.windowManager.session = singleton {
      name = "bunnuafeth";
      start = ''
        ${pkgs.bunnuafeth}/bin/bunnu &
        waitPID=$!
      '';
    };
    environment.systemPackages = [ pkgs.bunnuafeth ];
  };
}
