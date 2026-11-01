#pragma once
#include <stdint.h>

// The board's own screen, shown while no app is connected: a light wave crosses the 6x6 grid and
// leaves the D lit, the wordmark slides in, then three pads blink in turn while it waits.
// `ms` is the time since the screen appeared.
void drawLogoScreen(uint8_t* frame, uint32_t ms);

// The same wave on the real pads, 0-35
bool logoPadLit(int pad, uint32_t ms);
