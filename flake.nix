{
  description = "Rust OS Kernel Development Flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, rust-overlay, flake-utils, ... }: 
  let
    system = "x86_64-linux";
    overlays = [ (import rust-overlay) ];
    pkgs = import nixpkgs { inherit system overlays; };
    pkgs-unstable = import nixpkgs-unstable { inherit system overlays; };

    rust = pkgs.rust-bin.nightly."2025-03-03".default.override {
      targets = [ "x86_64-unknown-none" ];
    };

  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = [
        (pkgs-unstable.limine.override {
          enableAll = true;
        })
        rust
        pkgs.qemu
        pkgs.gdb
        pkgs.nasm
        pkgs.rust-analyzer
        pkgs.clippy
      ];

      shellHook = ''
        exec zsh -c "nvim"
      '';
    };
  };
}

