#!/bin/bash

if [ -t 0 ] ; then
    echo "stdin is atty"
else
    echo "stdin is not atty"
fi

if [ -t 1 ] ; then
    echo "stdout is atty"
else
    echo "stdout is not atty"
fi

if [ -t 2 ] ; then
    echo "stderr is atty"
    echo "terminal size: $(stty size)"
else
    echo "stderr is not atty"
fi

>&2 echo "This is an error message."

true
