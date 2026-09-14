{
  alsa-lib,
  pkg-config,
  libudev-zero,
  wayland,
  cmake,
  vulkan-loader,
  stdenv,
  cargo-watch,
  mold,
  pkgs ? import {},
}: let
  libPath = with pkgs;
    lib.makeLibraryPath [
      wayland
      alsa-lib
      libxkbcommon
      vulkan-loader
    ];
in
  stdenv.mkDerivation (finalAttrs: {
    name = "Aim Trainer";
    src = ./.;
    buildInputs = [
      alsa-lib
      libudev-zero
      pkg-config
      cmake
      vulkan-loader
      wayland
      cargo-watch
      mold
    ];
    LD_LIBRARY_PATH = libPath;
    dontConfigure = true;
  })
