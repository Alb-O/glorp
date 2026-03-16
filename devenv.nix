{
  inputs,
  pkgs,
  lib,
  ...
}:

let
  linuxRuntimeLibs = with pkgs; [
    alsa-lib
    libX11
    libXcursor
    libXi
    libXrandr
    libxkbcommon
    mesa
    vulkan-loader
    wayland
  ];
  fontsConf = pkgs.makeFontsConf {
    fontDirectories = [
      pkgs.jetbrains-mono
      pkgs.noto-fonts
      pkgs.noto-fonts-cjk-sans
    ];
  };
in
{
  imports = [ (inputs.poly-rust-env + "/devenv.nix") ];

  rustEnv.managedCargo.enable = true;

  env = lib.optionalAttrs pkgs.stdenv.isLinux {
    FONTCONFIG_FILE = fontsConf;
    LD_LIBRARY_PATH = lib.concatStringsSep ":" [
      (lib.makeLibraryPath linuxRuntimeLibs)
      "/run/opengl-driver/lib"
    ];
  };

  packages = [
    pkgs.fontconfig
    pkgs.jetbrains-mono
    pkgs.noto-fonts
    pkgs.noto-fonts-cjk-sans
  ]
  ++ lib.optionals pkgs.stdenv.isLinux linuxRuntimeLibs;

  scripts = {
    run-app.exec = ''
      cargo run
    '';

    check-targets.exec = lib.mkForce ''
      cargo check --all-targets --all-features
    '';
  };

  enterShell = ''
    echo "Run: cargo run"
    echo "Run: check-targets"
  '';
}
