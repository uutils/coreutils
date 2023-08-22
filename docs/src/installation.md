<!-- spell-checker:ignore pacman pamac nixpkgs openmandriva conda -->

# Installation

This is a list of uutils packages in various distributions and package managers.
Note that these are packaged by third-parties and the packages might contain
patches.

You can also [build uutils from source](/build.md).

<!-- toc -->

## Cargo

[![crates.io package](https://repology.org/badge/version-for-repo/crates_io/uutils-coreutils.svg)](https://repology.org/project/uutils-coreutils/versions)

```shell
# Linux
cargo install coreutils --features unix
# MacOs
cargo install coreutils --features macos
# Windows
cargo install coreutils --features windows
```

## Linux

### Alpine

[![Alpine Linux Edge package](https://repology.org/badge/version-for-repo/alpine_edge/uutils-coreutils.svg)](https://pkgs.alpinelinux.org/packages?name=uutils-coreutils)

```shell
apk update uutils-coreutils
```

> **Note**: Requires the `edge` repository.

### Arch

[![Arch package](https://repology.org/badge/version-for-repo/arch/uutils-coreutils.svg)](https://archlinux.org/packages/extra/x86_64/uutils-coreutils/)

```shell
pacman -S uutils-coreutils
```

### Debian

[![Debian package](https://repology.org/badge/version-for-repo/debian_unstable/uutils-coreutils.svg)](https://packages.debian.org/sid/source/rust-coreutils)

```shell
apt install rust-coreutils
# To use it:
export PATH=/usr/lib/cargo/bin/coreutils:$PATH
```

> **Note**: Only available from Bookworm (Debian 12)

### Gentoo

[![Gentoo package](https://repology.org/badge/version-for-repo/gentoo/uutils-coreutils.svg)](https://packages.gentoo.org/packages/sys-apps/uutils-coreutils)

```shell
emerge -pv sys-apps/uutils-coreutils
```

### Manjaro

![Manjaro Stable package](https://repology.org/badge/version-for-repo/manjaro_stable/uutils-coreutils.svg)
[![Manjaro Testing package](https://repology.org/badge/version-for-repo/manjaro_testing/uutils-coreutils.svg)](https://repology.org/project/uutils-coreutils/versions)
[![Manjaro Unstable package](https://repology.org/badge/version-for-repo/manjaro_unstable/uutils-coreutils.svg)](https://repology.org/project/uutils-coreutils/versions)

```shell
pacman -S uutils-coreutils
# or
pamac install uutils-coreutils
```

### NixOS

[![nixpkgs unstable package](https://repology.org/badge/version-for-repo/nix_unstable/uutils-coreutils.svg)](https://repology.org/project/uutils-coreutils/versions)

```shell
nix-env -iA nixos.uutils-coreutils
```

### OpenMandriva Lx

[![openmandriva cooker package](https://repology.org/badge/version-for-repo/openmandriva_cooker/uutils-coreutils.svg)](https://repology.org/project/uutils-coreutils/versions)

```shell
dnf install uutils-coreutils
```

### Ubuntu

[![Ubuntu package](https://repology.org/badge/version-for-repo/ubuntu_23_04/uutils-coreutils.svg)](https://packages.ubuntu.com/source/lunar/rust-coreutils)

```shell
apt install rust-coreutils
# To use it:
export PATH=/usr/lib/cargo/bin/coreutils:$PATH
```

> **Note**: Only available from Kinetic (Ubuntu 22.10)

## MacOS

### Homebrew

[![Homebrew package](https://repology.org/badge/version-for-repo/homebrew/uutils-coreutils.svg)](https://formulae.brew.sh/formula/uutils-coreutils)

```shell
brew install uutils-coreutils
```

### MacPorts

[![MacPorts package](https://repology.org/badge/version-for-repo/macports/uutils-coreutils.svg)](https://ports.macports.org/port/coreutils-uutils/)

```
port install coreutils-uutils
```

## FreeBSD

[![FreeBSD port](https://repology.org/badge/version-for-repo/freebsd/uutils-coreutils.svg)](https://repology.org/project/uutils-coreutils/versions)

```sh
pkg install rust-coreutils
```

## Windows

### Scoop

[![Scoop package](https://repology.org/badge/version-for-repo/scoop/uutils-coreutils.svg)](https://scoop.sh/#/apps?q=uutils-coreutils&s=0&d=1&o=true)

```shell
scoop install uutils-coreutils
```

## Alternative installers

### Conda

[Conda package](https://anaconda.org/conda-forge/uutils-coreutils)

```
conda install -c conda-forge uutils-coreutils 
```

## Non-standard packages

### `coreutils-hybrid` (AUR)

[![AUR package](https://repology.org/badge/version-for-repo/aur/coreutils-hybrid.svg)](https://aur.archlinux.org/packages/coreutils-hybrid)

A GNU coreutils / uutils coreutils hybrid package. Uses stable uutils
programs mixed with GNU counterparts if uutils counterpart is
unfinished or buggy.
