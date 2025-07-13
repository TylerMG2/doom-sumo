{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    rust-overlay,
  }:
    utils.lib.eachDefaultSystem (
      system: let
        buildTarget = "wasm32-unknown-unknown";

        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
          config = {
            allowUnfree = true;
          };
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [buildTarget];
          extensions = ["rust-src"];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.openssl
            pkgs.cargo-watch
            pkgs.wasm-pack
            pkgs.wasm-bindgen-cli
            pkgs.alsa-lib
            pkgs.rust-analyzer
            pkgs.udev

            # Keyboard input
            pkgs.libxkbcommon

            # Required by wayland-sys crate (via bevy)
            pkgs.wayland.dev

            # Graphics stuff
            pkgs.vulkan-loader
            pkgs.vulkan-tools
            pkgs.vulkan-validation-layers

            pkgs.linuxPackages.nvidia_x11

            # Add wasm-server-runner from source
            (rustPlatform.buildRustPackage {
              pname = "wasm-server-runner";
              version = "1.0.0";
              src = pkgs.fetchFromGitHub {
                owner = "jakobhellermann";
                repo = "wasm-server-runner";
                rev = "v1.0.0";
                sha256 = "sha256-3ARVVA+W9IS+kpV8j5lL/z6/ZImDaA+m0iEEQ2mSiTw=";
              };
              cargoHash = "sha256-FrnCmfSRAePZuWLC1/iRJ87CwLtgWRpbk6nJLyQQIT8=";
            })

            # Matchbox server for wasm
            (rustPlatform.buildRustPackage {
              pname = "matchbox-server";
              version = "0.12.0";
              src = pkgs.fetchFromGitHub {
                owner = "johanhelsing";
                repo = "matchbox";
                rev = "v0.12.0";
                sha256 = "sha256-zyA7qkBFuIEL+jxnfRzt82kEl/P5xIRySA3V0Qv2cN0=";
              };
              cargoBuildFlags = ["-p" "matchbox_server"];
              doCheck = false;
              cargoHash = "sha256-sbcZWLEgRga4iYiB2lBlZsztNjQzWWEIJHfzHovw3wU=";
            })

            # Bevy CLI
            (rustPlatform.buildRustPackage {
              pname = "bevy-cli";
              version = "unstable";

              nativeBuildInputs = [pkgs.pkg-config pkgs.lld];
              buildInputs = [pkgs.openssl];

              src = pkgs.fetchFromGitHub {
                owner = "TheBevyFlock";
                repo = "bevy_cli";
                rev = "cli-v0.1.0-alpha.1";
                sha256 = "sha256-v7BcmrG3/Ep+W5GkyKRD1kJ1nUxpxYlGGW3SNKh0U+8=";
              };

              cargoBuildFlags = ["--locked"];
              cargoHash = "sha256-QrW0daIjuFQ6Khl+3sTKM0FPGz6lMiRXw0RKXGZIHC0=";
              doCheck = false; # No network access in nix builds
            })
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.vulkan-loader
            pkgs.linuxPackages.nvidia_x11
            pkgs.udev
            pkgs.openssl
            pkgs.alsa-lib
            pkgs.wayland
            pkgs.libxkbcommon
          ];

          shellHook = ''
            echo "Welcome to the doomsumo development shell"
          '';
        };
      }
    );
}
