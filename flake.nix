{
  description = "recogedor cursed IMAP forwarding service";

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
      inherit (pkgs) lib;
      cargoToml = lib.importTOML ./Cargo.toml;
    in rec {
      formatter = pkgs.alejandra;

      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = cargoToml.package.name;
        version = cargoToml.package.version;
        src = ./.;

        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = [
          pkgs.pkg-config
        ];

        buildInputs =
          [
            pkgs.openssl
          ]
          ++ lib.optionals (pkgs.stdenv.isDarwin) [
            pkgs.darwin.apple_sdk.frameworks.Security
          ];
      };

      devShells.default = packages.default.overrideAttrs (finalAttrs: prevAttrs: {
        nativeBuildInputs =
          prevAttrs.nativeBuildInputs
          ++ [
            pkgs.rust-analyzer
            pkgs.lldb_16
          ];
      });

      nixosModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.services.recogedor;
        inherit (lib) mkIf mkEnableOption mkOption types;
        tomlFormat = pkgs.formats.toml {};
      in {
        options.services.recogedor = {
          enable = mkEnableOption "Enable the recogedor cursed IMAP forwarding service";

          logLevel = mkOption {
            type = types.nullOr (types.enum ["error" "warn" "info" "debug" "trace"]);
            default = "info";
            description = "Minimum log level.";
          };

          settings = mkOption {
            type = tomlFormat.type;
            default = {};
            description = "config.toml file used by recogedor.";
          };

          package = mkOption {
            type = types.package;
            default = self.packages.${system}.default;
            description = "Package to use for recogedor (defaults to this flake's).";
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
            environment = lib.optionalAttrs (cfg.logLevel != null) {
              RUST_LOG = cfg.logLevel;
            };
          };
        });
      };
    });
}
