{
  description = "recogedor";

  inputs = {
    rust-overlay.url = github:oxalica/rust-overlay;
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {inherit system overlays;};
    in rec {
      formatter = pkgs.alejandra;

      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "recogedor";
        version = "0.0.1";
        src = ./.;

        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.openssl
        ];

        buildInputs = pkgs.lib.optionals (pkgs.stdenv.isDarwin) [
          pkgs.darwin.apple_sdk.frameworks.Security
        ];
      };

      devShells.default = pkgs.mkShell {
        buildInputs =
          [
            pkgs.rust-bin.stable.latest.default
            pkgs.rust-analyzer
            pkgs.lldb_16
            pkgs.openssl
          ]
          ++ pkgs.lib.optionals (pkgs.stdenv.isDarwin) [
            pkgs.darwin.apple_sdk.frameworks.Security
          ];
      };

      devShells.production = pkgs.mkShell {
        buildInputs = [
          pkgs.rust-bin.stable.latest.default
          pkgs.pkg-config
          pkgs.openssl
        ];
      };

      nixosModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.services.kivikakk.recogedor;
        inherit (lib) mkIf mkEnableOption mkOption;
        tomlFormat = pkgs.formats.toml {};
      in {
        options.services.kivikakk.recogedor = {
          enable = mkEnableOption "Enable the recogedor IMAP forwarding service";

          settings = mkOption {
            type = tomlFormat.type;
            default = {};
            description = ''
              config.toml file used by recogedor.
            '';
          };

          package = mkOption {
            type = lib.types.package;
            default = self.packages.${system}.default;
            description = "package to use for recogedor (defaults to this flake's)";
          };
        };

        config = mkIf (cfg.enable) (let
          configFile = tomlFormat.generate "config.toml" cfg.settings;
        in {
          systemd.services.recogedor = {
            description = "recogedor IMAP forwarding service";
            wantedBy = ["multi-user.target"];

            serviceConfig = {
              DynamicUser = "yes";
              ExecStart = "${cfg.package}/bin/recogedor --config ${configFile}";
              Restart = "on-failure";
              RestartSec = "5s";
            };
          };
        });
      };
    });
}
