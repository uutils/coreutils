# spell-checker:ignore bintools gnum gperf ldflags libclang nixpkgs numtide pkgs texinfo
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      libselinuxPath = with pkgs;
        lib.makeLibraryPath [
          libselinux
        ];
      libaclPath = with pkgs;
        lib.makeLibraryPath [
          acl
        ];

      build_deps = with pkgs; [
          clang
          llvmPackages.bintools
          rustup

          pre-commit

          # debugging
          gdb
        ];
      gnu_testing_deps = with pkgs; [
          autoconf
          automake
          bison
          gnum4
          gperf
          gettext
          texinfo
        ];
    in {
      devShell = pkgs.mkShell {
        buildInputs = build_deps ++ gnu_testing_deps;

        RUSTC_VERSION = "1.75";
        LIBCLANG_PATH = pkgs.lib.makeLibraryPath [pkgs.llvmPackages_latest.libclang.lib];
        shellHook = ''
          export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
          export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
        '';

        SELINUX_INCLUDE_DIR = ''${pkgs.libselinux.dev}/include'';
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
    });
}
