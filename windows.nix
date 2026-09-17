{
  pkg-config,
  stdenv,
  cargo-xwin,
  libclang,
  pkgs,
}: let
  libPath = with pkgs;
    lib.makeLibraryPath [
    ];
in
  stdenv.mkDerivation (finalAttrs: {
    name = "Aim Trainer";
    src = ./.;

    nativeBuildInputs = [
      cargo-xwin
      libclang
      pkg-config
    ];
    LD_LIBRARY_PATH = libPath;
    dontConfigure = true;
  })
