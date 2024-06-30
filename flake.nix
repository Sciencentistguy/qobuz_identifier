{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = github:edolstra/flake-compat;
      flake = false;
    };
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
    {
      overlay = final: prev: {
        qobuz-identifier = self.packages.${prev.system}.default;
      };
    }
    // flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;
        qobuz-identifier = {
          lib,
          openssl,
          pkg-config,
          rustPlatform,
          stdenv,
          darwin,
        }:
          rustPlatform.buildRustPackage {
            name = "qobuz-identifier";
            src = lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [
              pkg-config
              rustPlatform.bindgenHook
            ];
            buildInputs =
              [openssl]
              ++ lib.optionals stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
                Security
                SystemConfiguration
              ]);
            postInstall = ''
              mv $out/bin/qobuz_identifier $out/bin/qobuz-identifier
            '';
            meta = with lib; {
              license = licenses.mpl20;
              homepage = "https://github.com/Sciencentistguy/qobuz-identifier";
              platforms = platforms.all;
            };
          };
      in {
        packages.qobuz-identifier = pkgs.callPackage qobuz-identifier {};

        packages.default = self.packages.${system}.qobuz-identifier;
        devShells.default = self.packages.${system}.default.overrideAttrs (super: {
          nativeBuildInputs = with pkgs;
            super.nativeBuildInputs
            ++ [
              cargo-edit
              clippy
              rustfmt
            ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        });
      }
    );
}
