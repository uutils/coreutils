#!/bin/bash

cd $(dirname $0)
rustc libstdbuf.rs
gcc -c -Wall -Werror -fpic libstdbuf.c -L. -llibstdbuf.so
gcc -shared -o libstdbuf.so libstdbuf.o
mv *.so ../../build/
rm *.o
export LD_LIBRARY_PATH="$LD_LIBRARY_PATH":"$PWD"/../../build
