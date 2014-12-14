#!/bin/bash

cd $(dirname $0)
rustc libstdbuf.rs
gcc -c -Wall -Werror -fpic libstdbuf.c -L. -llibstdbuf.a
gcc -shared -o libstdbuf.so -Wl,--whole-archive liblibstdbuf.a -Wl,--no-whole-archive libstdbuf.o -lpthread
mv *.so ../../build/
rm *.o
rm *.a
