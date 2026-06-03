#!/bin/bash

if [ -t 0 ] ; then
    echo "stdin is a tty"
    echo "terminal size: $(stty size)"
else
    echo "stdin is not a tty"
fi

if [ -t 1 ] ; then
    echo "stdout is a tty"
else
    echo "stdout is not a tty"
fi

if [ -t 2 ] ; then
    echo "stderr is a tty"
else
    echo "stderr is not a tty"
fi

>&2 echo "This is an error message."

true
