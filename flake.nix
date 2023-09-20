{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, flake-parts, rust-overlay, crane, nixpkgs, ... }:
    let
      GIT_HASH = self.shortRev or self.dirtyShortRev;
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = { pkgs, system, ... }:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };

          commonArgs = {
            src = craneLib.cleanCargoSource (craneLib.path ./.);

            buildInputs = with pkgs; [
              mold
              clang
              xorg.libX11
              xorg.libXft
              xorg.libXrandr
              xorg.libXinerama
            ];

            inherit GIT_HASH;
          } // (craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; });

          craneLib = (crane.mkLib pkgs).overrideToolchain pkgs.rust-bin.stable.latest.minimal;

          cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
            panme = "bunnuafeth-deps";
          });

          bunnuafeth = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;

            postFixup = ''
                patchelf --set-rpath "${pkgs.lib.makeLibraryPath commonArgs.buildInputs}" $out/bin/bunnuafeth
            '';

            NIX_CFLAGS_LINK = "-fuse-ld=mold";
          });
        in
        {

          # `nix build`
          packages = {
            inherit bunnuafeth;
            default = bunnuafeth;
          };

          # `nix develop`
          devShells.default = pkgs.mkShell
            {
              NIX_CFLAGS_LINK = "-fuse-ld=mold";

              packages = with pkgs; [
                (rust-bin.stable.latest.default.override {
                  extensions = [ "rust-src" "rust-analyzer" ];
                })
              ];

              buildInputs = with pkgs;[
                pkg-config
                systemd
              ] ++ commonArgs.buildInputs;

              nativeBuildInputs = with pkgs; [
                virt-viewer
              ];

              shellHook = ''
                source .nixos-vm/vm.sh
              '';

              inherit GIT_HASH;

              LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [ pkgs.xorg.libX11 pkgs.xorg.libXcursor pkgs.xorg.libXrandr pkgs.xorg.libXi pkgs.xorg.libXft ]}";
            };
        };

      flake = {
        formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixpkgs-fmt;
        overlays.default = final: prev: {
          bunnuafeth = self.packages.${final.system}.bunnuafeth;
        };

        # nixos development vm
        nixosConfigurations.bunnuafeth = nixpkgs.lib.nixosSystem
          {
            system = "x86_64-linux";
            modules = [
              {
                nixpkgs.overlays = [
                  self.overlays.default
                ];
              }
              "${nixpkgs}/nixos/modules/virtualisation/qemu-vm.nix"
              ./.nixos-vm/configuration.nix
            ];
          };
      };
    };
}

