#pragma once
#include <stdint.h>

const int DISPLAY_FRAME_SIZE = 1024;  // 128x64, 1 bit per pixel, SH1106 page layout

bool displayBegin();  // false when no display answers on the bus
void displayShow(const uint8_t* frame);
bool displayWake();   // true if the display was off
bool displayIsOff();
void displaySuspend(bool suspended);
void displayUpdate();
