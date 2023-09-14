{ config, pkgs, ... }:
{
  system.stateVersion = "23.05";
  boot.initrd.availableKernelModules = [ "virtio_net" "virtio_pci" "virtio_mmio" "virtio_blk" "virtio_scsi" "9p" "9pnet_virtio" ];
  boot.initrd.kernelModules = [ "virtio_balloon" "virtio_console" "virtio_rng" ];

  virtualisation.diskImage = "./.nixos-vm/${config.system.name}.qcow2";

  users.users.bunnuafeth = {
    isNormalUser = true;
    extraGroups = [ "wheel" ];
    initialPassword = "";
    home = "/home/bunnuafeth";
  };

  networking = {
    hostName = "bunnuafeth";
    networkmanager.enable = true;
  };

  environment.systemPackages = with pkgs; [
    bunnuafeth
    alacritty
    polybar
    lshw
    magic-wormhole
    arandr
    kitty
    jq
    vim
  ];

  programs.git.enable = true;
  services = {
    openssh.enable = true;
    spice-vdagentd.enable = true;
    qemuGuest.enable = true;

    xserver = {
      videoDrivers = [ "qxl" ];
      enable = true;

      # desktopManager.xterm.enable = false;
      # displayManager.autoLogin.enable = true;
      # displayManager.autoLogin.user = "bunnuafeth";
      # windowManager.bunnuafeth.enable = true;
    };
  };

}
