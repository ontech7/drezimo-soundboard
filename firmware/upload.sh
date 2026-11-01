#!/bin/sh
# Builds the firmware and uploads it to the connected board, whether it is running or already
# in bootloader mode (RPI-RP2).
set -e
cd "$(dirname "$0")"

# --clean: the cached core would otherwise keep the USB name it was first built with
arduino-cli compile --profile rp2040-zero --output-dir build --clean \
  --build-property 'build.usb_manufacturer="Drezimo"' \
  --build-property 'build.usb_product="Drezimo Soundboard"' \
  drezimo

port=$(ls /dev/cu.usbmodem* 2>/dev/null | head -n 1 || true)
if [ -n "$port" ]; then
  printf 'BOOT\n' > "$port"
  sleep 3
fi

arduino-cli upload --profile rp2040-zero --input-dir build drezimo
