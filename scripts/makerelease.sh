#!/bin/sh
# -*- coding: utf-8 -*-

srcdir="$(realpath "$0" | xargs dirname)"
srcdir="$srcdir/.."

# Import the makerelease.lib
# https://bues.ch/cgit/misc.git/tree/makerelease.lib
die() { echo "$*"; exit 1; }
for path in $(echo "$PATH" | tr ':' ' '); do
    [ -f "$MAKERELEASE_LIB" ] && break
    MAKERELEASE_LIB="$path/makerelease.lib"
done
[ -f "$MAKERELEASE_LIB" ] && . "$MAKERELEASE_LIB" || die "makerelease.lib not found."

hook_get_version()
{
    cd "$1"
    version="$(cargo_local_pkg_version avr-context)"
}

project=avr-context
conf_upload_packages="avr-context"
makerelease "$@"

# vim: ts=4 sw=4 expandtab
