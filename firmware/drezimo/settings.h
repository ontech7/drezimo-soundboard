#pragma once
#include <stdint.h>

struct Settings {
  uint16_t oledBrightness;  // 0-100
  uint16_t oledDimAfter;    // minutes without input, 0 = never
  uint16_t oledOffAfter;    // minutes without input, 0 = never
  uint16_t oledShiftEvery;  // minutes, 0 = off
};

extern Settings settings;

void settingsLoad();
bool settingsSet(const char* name, int value);  // false for an unknown name or a value out of range
