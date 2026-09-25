{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  profiles = {
    linux.module = {
      packages = with pkgs; [
        alsa-lib
        libudev-zero
        pkg-config
        cmake
        vulkan-loader
        wayland
        cargo-watch
        cargo-xwin
        mold
        libxkbcommon
      ];
      env.LD_LIBRARY_PATH = with pkgs;
      lib.makeLibraryPath [
        wayland
        alsa-lib
        libxkbcommon
        vulkan-loader
      ];
    };

  };
  languages.rust = {
    enable = true;
    channel = "nightly";
    lsp.enable = true;
    mold.enable = true;
  };

  # https://devenv.sh/basics/
  enterShell = ''

  '';
}
