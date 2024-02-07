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
else
    echo "stderr is not atty"
fi

true
