#!/usr/bin/env bash
# Makes the headers of KDecoration3 available to build the helper when the
# system does not have them (the package libkdecorations3-dev). The packages are
# downloaded and unpacked in style-preview/build/deps: nothing is installed in the
# system and no administrator password is asked for.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
deps="$here/build/deps"

# The system has them: nothing to do.
if compgen -G "/usr/lib/*/cmake/KDecoration3/KDecoration3Config.cmake" >/dev/null \
    || compgen -G "/usr/lib/cmake/KDecoration3/KDecoration3Config.cmake" >/dev/null; then
    exit 0
fi

# Already unpacked by a previous build.
if compgen -G "$deps/usr/lib/*/cmake/KDecoration3/KDecoration3Config.cmake" >/dev/null; then
    exit 0
fi

mkdir -p "$deps/debs"
cd "$deps/debs"
# The three have the same version: the headers, the library and its private part.
apt-get download libkdecorations3-dev libkdecorations3-6 libkdecorations3private2
for package in *.deb; do
    dpkg -x "$package" "$deps"
done
