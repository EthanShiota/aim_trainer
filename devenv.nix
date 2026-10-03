{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  packages = with pkgs; [
    alsa-lib
    libudev-zero
    pkg-config
    cmake
    vulkan-loader
    vulkan-tools
    wayland
    cargo-watch
    cargo-xwin
    mold
    libxkbcommon
  ];
  env.LD_LIBRARY_PATH = with pkgs;
  lib.makeLibraryPath [
      vulkan-loader
      libxcursor
      libxkbcommon
      wayland
  ];
  languages.rust = {
    enable = true;
    channel = "nightly";
    lsp.enable = true;
    mold.enable = true;
  };

}
