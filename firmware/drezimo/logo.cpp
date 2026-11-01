#include <string.h>
#include "logo.h"
#include "logo_bitmap.h"

namespace {

const int WIDTH = 128;
const int FRAME_SIZE = WIDTH * 64 / 8;
const int GRID_SIZE = 6;

const uint32_t WAVE_STEP_MS = 45;    // from one diagonal of pads to the next
const uint32_t WAVE_CREST_STEPS = 2;
const uint32_t WORDMARK_START_MS = 400;
const uint32_t WORDMARK_MS = 350;
const uint32_t WAITING_START_MS = 1300;
const uint32_t WAITING_STEP_MS = 400;
const int WAITING_PADS = 3;
const int WAITING_PITCH = 8;
const int WAITING_Y = 53;

enum class Pad { Hidden, Dot, Lit };

bool isPartOfD(int row, int column) {
  return LOGO_D_ROWS[row] >> column & 1;
}

Pad padAt(int row, int column, uint32_t ms) {
  uint32_t step = ms / WAVE_STEP_MS;
  uint32_t reachedAt = row + column;
  if (step < reachedAt) {
    return Pad::Hidden;
  }
  if (step < reachedAt + WAVE_CREST_STEPS || isPartOfD(row, column)) {
    return Pad::Lit;
  }
  return Pad::Dot;
}

void fillRect(uint8_t* frame, int x, int y, int width, int height) {
  for (int row = y; row < y + height; row++) {
    for (int column = x; column < x + width; column++) {
      frame[row / 8 * WIDTH + column] |= 1 << (row % 8);
    }
  }
}

void drawPad(uint8_t* frame, int x, int y, Pad pad) {
  const int dotInset = (LOGO_PAD_SIZE - LOGO_DOT_SIZE) / 2;
  if (pad == Pad::Lit) {
    fillRect(frame, x, y, LOGO_PAD_SIZE, LOGO_PAD_SIZE);
  } else if (pad == Pad::Dot) {
    fillRect(frame, x + dotInset, y + dotInset, LOGO_DOT_SIZE, LOGO_DOT_SIZE);
  }
}

void drawWordmark(uint8_t* frame, uint32_t ms) {
  if (ms < WORDMARK_START_MS) {
    return;
  }
  int width = LOGO_WORDMARK_RIGHT + 1 - LOGO_WORDMARK_LEFT;
  uint32_t elapsed = ms - WORDMARK_START_MS;
  int shown = elapsed >= WORDMARK_MS ? width : elapsed * width / WORDMARK_MS;
  for (int page = 0; page < 8; page++) {
    int start = page * WIDTH + LOGO_WORDMARK_LEFT;
    memcpy(frame + start, LOGO_WORDMARK + start, shown);
  }
}

void drawWaitingPads(uint8_t* frame, uint32_t ms) {
  int width = (WAITING_PADS - 1) * WAITING_PITCH + LOGO_PAD_SIZE;
  int x = (WIDTH - width) / 2;
  int lit = ms / WAITING_STEP_MS % WAITING_PADS;
  for (int i = 0; i < WAITING_PADS; i++) {
    drawPad(frame, x + i * WAITING_PITCH, WAITING_Y, i == lit ? Pad::Lit : Pad::Dot);
  }
}

}  // namespace

void drawLogoScreen(uint8_t* frame, uint32_t ms) {
  memset(frame, 0, FRAME_SIZE);
  for (int row = 0; row < GRID_SIZE; row++) {
    for (int column = 0; column < GRID_SIZE; column++) {
      int x = LOGO_GRID_X + column * LOGO_PAD_PITCH;
      int y = LOGO_GRID_Y + row * LOGO_PAD_PITCH;
      drawPad(frame, x, y, padAt(row, column, ms));
    }
  }
  drawWordmark(frame, ms);
  if (ms >= WAITING_START_MS) {
    drawWaitingPads(frame, ms - WAITING_START_MS);
  }
}

bool logoPadLit(int pad, uint32_t ms) {
  return padAt(pad / GRID_SIZE, pad % GRID_SIZE, ms) == Pad::Lit;
}
