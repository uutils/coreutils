#!/bin/bash

[ -t 0 ] && echo "stdin is a tty" || echo "stdin is not a tty"
[ -t 1 ] && echo "stdout is a tty" || echo "stdout is not a tty"
[ -t 2 ] && echo "stderr is a tty" || echo "stderr is not a tty"
:
