{
  description = "Development shell for recogedor";

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
    flake-utils.lib.eachDefaultSystem (system:
      with rec {
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
      }; rec {
        formatter = pkgs.alejandra;

        devShells.default = pkgs.mkShell {
          buildInputs = builtins.attrValues {
            rust = pkgs.rust-bin.stable.latest.default;
            security = pkgs.darwin.apple_sdk.frameworks.Security;
            inherit (pkgs) rust-analyzer lldb_16 openssl;
          };
        };

        devShells.production = pkgs.mkShell {
          buildInputs = builtins.attrValues {
            rust = pkgs.rust-bin.stable.latest.default;
            inherit (pkgs) openssl;
          };
        };
      });
}
