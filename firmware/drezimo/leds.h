#pragma once
#include <stdint.h>

const int LED_COUNT = 42;  // 0-35 pads, 36-41 function keys

enum class LedEffect { Solid, Breathe, Blink };

void ledsBegin();
void ledsSet(int index, uint8_t red, uint8_t green, uint8_t blue, LedEffect effect = LedEffect::Solid);
void ledsClear();
void ledsSetBrightness(int percent);
void ledsSuspend(bool suspended);
void ledsUpdate();
uint32_t ledsShown(int index);  // the colour the LED shows right now, as 0xRRGGBB
