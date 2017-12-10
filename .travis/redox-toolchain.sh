#!/bin/bash

rustup target add x86_64-unknown-redox
sudo apt-key adv --keyserver keyserver.ubuntu.com --recv-keys AA12E97F0881517F
sudo add-apt-repository 'deb https://static.redox-os.org/toolchain/apt /'
sudo apt-get update -qq
sudo apt-get install -y x86-64-unknown-redox-gcc
