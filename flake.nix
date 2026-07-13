{
  description = "Edge Tasks development environment (Rust, Vite+, Caddy, just)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-vite-plus = {
      url = "github:ryoppippi/nix-vite-plus";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      nix-vite-plus,
    }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [
                rust-overlay.overlays.default
                nix-vite-plus.overlays.default
              ];
            }
          )
        );
    in
    {
      formatter = forAllSystems (pkgs: pkgs.nixfmt);

      devShells = forAllSystems (
        pkgs:
        let
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rustfmt"
              "clippy"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.just
              pkgs.caddy
              pkgs.openssl
              pkgs.curl
              pkgs.lsof
              pkgs.vite-plus
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
              pkgs.podman
            ];

            shellHook = ''
              echo "Edge Tasks nix develop shell"
              echo "  just start | just status | just stop"
              if ! command -v podman >/dev/null 2>&1; then
                echo "warning: podman is not on PATH (install the host Podman app on macOS)" >&2
              fi
            '';
          };
        }
      );
    };
}
