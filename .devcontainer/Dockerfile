FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

# install gnu coreutils test dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       attr \
       autoconf \
       automake \
       autopoint \
       bison \
       g++ \
       gcc \
       gdb \
       gperf \
       jq \
       libacl1-dev \
       libattr1-dev \
       libcap-dev \
       libexpect-perl \
       libselinux1-dev \
       python3-pyinotify \
       quilt \
       texinfo \
       valgrind \
    && rm -rf /var/lib/apt/lists/*

# install dependencies for uutils
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        clang \
        gdb \
        python3-pip \
    && rm -rf /var/lib/apt/lists/*

# pre-commit
RUN pip3 install --break-system-packages pre-commit
