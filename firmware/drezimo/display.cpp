#include <U8g2lib.h>
#include <Wire.h>
#include "display.h"
#include "pins.h"
#include "settings.h"

namespace {

const int WIDTH = 128;
const int PAGES = 8;
const uint8_t I2C_ADDRESS = 0x3C;
const uint32_t MINUTE_MS = 60000;
const uint8_t DIMMED_CONTRAST = 0;

// Burn-in protection moves the picture around the 1 px margin the app leaves free
const int8_t SHIFTS[][2] = {{0, 0}, {1, 0}, {1, 1}, {0, 1}, {-1, 1}, {-1, 0}, {-1, -1}, {0, -1}, {1, -1}};
const int SHIFT_COUNT = sizeof(SHIFTS) / sizeof(SHIFTS[0]);

enum class Power { On, Dimmed, Off };

U8G2_SH1106_128X64_NONAME_F_HW_I2C oled(U8G2_R0);
bool present = false;
uint8_t picture[DISPLAY_FRAME_SIZE];
uint8_t dirtyPages = 0;
int shift = 0;
uint32_t shiftedAt = 0;
uint32_t activeAt = 0;
bool suspended = false;
Power power = Power::On;
int contrast = -1;

uint64_t columnBits(const uint8_t* frame, int x) {
  uint64_t bits = 0;
  for (int page = 0; page < PAGES; page++) {
    bits |= uint64_t(frame[page * WIDTH + x]) << (8 * page);
  }
  return bits;
}

// Copies the picture to the display buffer, shifted, and marks the pages that changed
void render() {
  uint8_t* buffer = oled.getBufferPtr();
  int dx = SHIFTS[shift][0];
  int dy = SHIFTS[shift][1];
  for (int x = 0; x < WIDTH; x++) {
    int source = x - dx;
    uint64_t bits = source >= 0 && source < WIDTH ? columnBits(picture, source) : 0;
    bits = dy >= 0 ? bits << dy : bits >> -dy;
    for (int page = 0; page < PAGES; page++) {
      uint8_t byte = bits >> (8 * page);
      uint8_t& shown = buffer[page * WIDTH + x];
      if (shown != byte) {
        shown = byte;
        dirtyPages |= 1 << page;
      }
    }
  }
}

Power wantedPower() {
  uint32_t idleMinutes = (millis() - activeAt) / MINUTE_MS;
  if (suspended || (settings.oledOffAfter > 0 && idleMinutes >= settings.oledOffAfter)) {
    return Power::Off;
  }
  if (settings.oledDimAfter > 0 && idleMinutes >= settings.oledDimAfter) {
    return Power::Dimmed;
  }
  return Power::On;
}

void updatePower() {
  Power wanted = wantedPower();
  int wantedContrast = wanted == Power::Dimmed ? DIMMED_CONTRAST : settings.oledBrightness * 255 / 100;
  if (wantedContrast != contrast) {
    oled.setContrast(wantedContrast);
    contrast = wantedContrast;
  }
  if (wanted != power) {
    oled.setPowerSave(wanted == Power::Off);
    power = wanted;
  }
}

void updateShift() {
  int wantedShift = shift;
  if (settings.oledShiftEvery == 0) {
    wantedShift = 0;
  } else if (millis() - shiftedAt >= settings.oledShiftEvery * MINUTE_MS) {
    wantedShift = (shift + 1) % SHIFT_COUNT;
    shiftedAt = millis();
  }
  if (wantedShift != shift) {
    shift = wantedShift;
    render();
  }
}

// One page (128x8) takes about 3 ms over I2C: sending one per call keeps the key scan responsive
void sendOnePage() {
  for (int page = 0; page < PAGES; page++) {
    if (dirtyPages & (1 << page)) {
      dirtyPages &= ~(1 << page);
      oled.updateDisplayArea(0, page, WIDTH / 8, 1);
      return;
    }
  }
}

}  // namespace

bool displayBegin() {
  Wire.setSDA(OLED_SDA_PIN);
  Wire.setSCL(OLED_SCL_PIN);
  Wire.begin();
  Wire.beginTransmission(I2C_ADDRESS);
  present = Wire.endTransmission() == 0;
  if (!present) {
    return false;
  }

  oled.setBusClock(400000);
  oled.begin();
  activeAt = millis();
  shiftedAt = millis();
  return true;
}

void displayShow(const uint8_t* frame) {
  if (!present) {
    return;
  }
  memcpy(picture, frame, DISPLAY_FRAME_SIZE);
  render();
}

bool displayWake() {
  bool wasOff = displayIsOff();
  activeAt = millis();
  return wasOff;
}

bool displayIsOff() {
  return present && power == Power::Off;
}

void displaySuspend(bool value) {
  suspended = value;
}

void displayUpdate() {
  if (!present) {
    return;
  }
  updatePower();
  updateShift();
  sendOnePage();
}
