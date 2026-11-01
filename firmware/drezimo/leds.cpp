#include <Adafruit_NeoPixel.h>
#include "leds.h"
#include "pins.h"

namespace {

const uint32_t FRAME_MS = 10;
const uint32_t BREATHE_PERIOD_MS = 1400;
const uint32_t BLINK_PERIOD_MS = 800;

// SK6812MINI-E: up to 12 mA per colour channel, about 1 mA when dark. USB gives 500 mA in all.
const float CHANNEL_MAX_MA = 12;
const float LED_DARK_MA = 1;
const float BUDGET_MA = 400;

struct Led {
  uint8_t red, green, blue;
  LedEffect effect;
};

Adafruit_NeoPixel strip(LED_COUNT, LED_DATA_PIN, NEO_GRB + NEO_KHZ800);
Led leds[LED_COUNT];
float brightness = 0.5;
bool suspended = false;
uint32_t renderedAt = 0;

// The data chain runs through the 6 function keys first, then the 36 pads
int chainPosition(int index) {
  return index >= 36 ? index - 36 : index + 6;
}

float effectLevel(LedEffect effect, uint32_t now) {
  switch (effect) {
    case LedEffect::Breathe:
      return 0.25f + 0.75f * (0.5f + 0.5f * cosf(TWO_PI * (now % BREATHE_PERIOD_MS) / BREATHE_PERIOD_MS));
    case LedEffect::Blink:
      return now % BLINK_PERIOD_MS >= BLINK_PERIOD_MS / 2 ? 1 : 0;
    default:
      return 1;
  }
}

}  // namespace

void ledsBegin() {
  strip.begin();
  strip.show();
}

void ledsSet(int index, uint8_t red, uint8_t green, uint8_t blue, LedEffect effect) {
  leds[index] = {red, green, blue, effect};
}

void ledsClear() {
  for (Led& led : leds) {
    led = {0, 0, 0, LedEffect::Solid};
  }
}

void ledsSetBrightness(int percent) {
  brightness = percent / 100.0f;
}

void ledsSuspend(bool value) {
  suspended = value;
}

void ledsUpdate() {
  uint32_t now = millis();
  if (now - renderedAt < FRAME_MS) {
    return;
  }
  renderedAt = now;

  float channels[LED_COUNT][3];
  float darkMa = LED_COUNT * LED_DARK_MA;
  float totalMa = darkMa;
  for (int i = 0; i < LED_COUNT; i++) {
    float level = suspended ? 0 : effectLevel(leds[i].effect, now) * brightness;
    channels[i][0] = strip.gamma8(leds[i].red) * level;
    channels[i][1] = strip.gamma8(leds[i].green) * level;
    channels[i][2] = strip.gamma8(leds[i].blue) * level;
    totalMa += (channels[i][0] + channels[i][1] + channels[i][2]) / 255 * CHANNEL_MAX_MA;
  }

  // Power limiter: dim everything evenly so the whole chain stays within the budget
  float limit = totalMa > BUDGET_MA ? (BUDGET_MA - darkMa) / (totalMa - darkMa) : 1;
  for (int i = 0; i < LED_COUNT; i++) {
    strip.setPixelColor(chainPosition(i), channels[i][0] * limit, channels[i][1] * limit, channels[i][2] * limit);
  }
  strip.show();
}

uint32_t ledsShown(int index) {
  return strip.getPixelColor(chainPosition(index));
}
