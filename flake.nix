{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    # geng.url = "/home/kuviman/projects/cargo-geng";
    geng.url = "github:geng-engine/cargo-geng";
    geng.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { geng, nixpkgs, rust-overlay, systems, ... }:
    let
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
        config = {
          allowUnfree = true;
          android_sdk.accept_license = true;
        };
      };
      forEachSystem = f: nixpkgs.lib.genAttrs (import systems) (system:
        let pkgs = pkgsFor system;
        in f system pkgs);
    in
    {
      devShells = forEachSystem (system: pkgs:
        {
          default = geng.lib.mkShell {
            inherit system;
            target.web.enable = true;
          };
        });
      formatter = forEachSystem (system: pkgs: pkgs.nixpkgs-fmt);
    };
}
