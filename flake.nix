{
  description = "Wallrus — Gnome (GTK4) application to generate colorful wallpapers based on gradients and different effects";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    supportedSystems = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
  in {
    packages = forAllSystems (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        wallrus = pkgs.callPackage ./nix/package.nix {};
        default = self.packages.${system}.wallrus;
      }
    );

    devShells = forAllSystems (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.callPackage ./nix/devShell.nix {
          wallrus = self.packages.${system}.wallrus;
        };
      }
    );
  };
}
