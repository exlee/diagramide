#!/bin/sh

rm -r tcl-sys/tcl8.6 tk-sys/tk8.6

mkdir tcl-sys/tcl8.6
mkdir tcl-sys/tcl8.6/generic
cp -r tcl/generic/*.h  tcl-sys/tcl8.6/generic

mkdir tk-sys/tk8.6 tk-sys/tk8.6/generic
cp -r tk/generic/*.h tk-sys/tk8.6/generic
