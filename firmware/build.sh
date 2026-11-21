#!/bin/sh
# Builds the firmware into build/. With --bundle, also copies it into the app, which then updates every board with an
# older firmware (docs/project.md, section 9).
set -e
cd "$(dirname "$0")"

# --clean: the cached core would otherwise keep the USB name it was first built with
arduino-cli compile --profile rp2040-zero --output-dir build --clean \
  --build-property 'build.usb_manufacturer="Drezimo"' \
  --build-property 'build.usb_product="Drezimo Soundboard"' \
  drezimo

if [ "$1" = "--bundle" ]; then
  version=$(sed -n 's/^const char FIRMWARE_VERSION\[\] = "\(.*\)";$/\1/p' drezimo/drezimo.ino)
  if [ -z "$version" ]; then
    echo "no FIRMWARE_VERSION line in drezimo/drezimo.ino" >&2
    exit 1
  fi
  bundled=../app/src-tauri/firmware
  mkdir -p "$bundled"
  rm -f "$bundled"/drezimo-*.uf2 "$bundled"/drezimo-*.bin
  # The .bin goes over USB serial to boards that take UPDATE, the .uf2 to the bootloader of older ones
  cp build/drezimo.ino.bin "$bundled/drezimo-$version.bin"
  cp build/drezimo.ino.uf2 "$bundled/drezimo-$version.uf2"
  echo "bundled firmware $version"
fi
