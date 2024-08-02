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
    in {
      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          clang
          llvmPackages.bintools
          rustup

          pre-commit

          # debugging
          gdb

          # For GNU testing
          autoconf
          automake
          bison
          gnum4
          gperf
          gettext
        ];
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

        #LD_LIBRARY_PATH = libselinuxPath;

        # Add glibc, clang, glib, and other headers to bindgen search path
        # BINDGEN_EXTRA_CLANG_ARGS =
        #   # Includes normal include path
        #   (builtins.map (a: ''-I"${a}/include"'') [
        #     # add dev libraries here (e.g. pkgs.libvmi.dev)
        #     pkgs.glibc.dev
        #     pkgs.libselinux.dev
        #   ])
        #   # Includes with special directory paths
        #   ++ [
        #     ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
        #     ''-I"${pkgs.glib.dev}/include/glib-2.0"''
        #     ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
        #   ];
      };
    });
}
