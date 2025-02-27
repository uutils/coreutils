# spell-checker:ignore bintools gnum gperf ldflags libclang nixpkgs numtide pkgs texinfo
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";

    # <https://github.com/nix-systems/nix-systems>
    systems.url = "github:nix-systems/default";
  };

  outputs = inputs: let
    eachSystem = inputs.nixpkgs.lib.genAttrs (import inputs.systems);
    pkgsFor = inputs.nixpkgs.legacyPackages;
  in {
    devShells = eachSystem (
      system: let
        inherit (pkgsFor.${system}) lib;

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

          RUSTC_VERSION = "1.79";
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
          RUSTFLAGS =
            (builtins.map (a: ''-L ${a}/lib'') [
              ])
            ++ [
              ''-L ${libselinuxPath}''
              ''-L ${libaclPath}''
            ];
        };
      }
    );
  };
}
