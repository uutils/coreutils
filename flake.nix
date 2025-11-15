# spell-checker:ignore bintools gnum gperf ldflags libclang nixpkgs numtide pkgs texinfo gettext
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";

    # <https://github.com/nix-systems/nix-systems>
    systems.url = "github:nix-systems/default";
  };

  outputs = inputs: let
    inherit (inputs.nixpkgs) lib legacyPackages;
    eachSystem = lib.genAttrs (import inputs.systems);
    pkgsFor = legacyPackages;
  in {
    devShells = eachSystem (
      system: let
        libselinuxPath = with pkgsFor.${system};
          lib.makeLibraryPath [
            libselinux
          ];

        libaclPath = with pkgsFor.${system};
          lib.makeLibraryPath [
            acl
          ];

        build_deps = with pkgsFor.${system}; [
          clang
          llvmPackages.bintools
          rustup

          pre-commit
          nodePackages.cspell

          # debugging
          gdb
        ];

        gnu_testing_deps = with pkgsFor.${system}; [
          autoconf
          automake
          bison
          gnum4
          gperf
          gettext
          texinfo
        ];
      in {
        default = pkgsFor.${system}.pkgs.mkShell {
          packages = build_deps ++ gnu_testing_deps;

          RUSTC_VERSION = "1.85";
          LIBCLANG_PATH = pkgsFor.${system}.lib.makeLibraryPath [pkgsFor.${system}.llvmPackages_latest.libclang.lib];
          shellHook = ''
            export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
            export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
          '';

          SELINUX_INCLUDE_DIR = ''${pkgsFor.${system}.libselinux.dev}/include'';
          SELINUX_LIB_DIR = libselinuxPath;
          SELINUX_STATIC = "0";

          # Necessary to build GNU.
          LDFLAGS = ''-L ${libselinuxPath} -L ${libaclPath}'';

          # Add precompiled library to rustc search path
          RUSTFLAGS = [
            ''-L ${libselinuxPath}''
            ''-L ${libaclPath}''
          ];
        };
      }
    );
  };
}
