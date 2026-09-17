{
  description = "Nix shell";

  inputs.nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
    };
    cross_pkgs = import nixpkgs {
      inherit system;
      config.allowUnfree = true;
      config.microsoftVisualStudioLicenseAccepted = true;
      # localsystem = system;
      # crossSystem = "x86_64-windows";
    };
  in {
    # TODO: Full build scripts
    packages.x86_64-linux = {
      default = pkgs.callPackage ./package.nix {};
      windows = cross_pkgs.callPackage ./windows.nix {};
    };
  };
}
