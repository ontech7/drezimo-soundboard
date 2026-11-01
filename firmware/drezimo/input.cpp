#include <Arduino.h>
#include "input.h"
#include "pins.h"

namespace {

const int COLUMNS = 6;
const int ROWS = 7;
const int KEY_COUNT = ROWS * COLUMNS + 1;  // the matrix plus the encoder button
const int ENCODER_BUTTON_KEY = KEY_COUNT - 1;
const uint32_t DEBOUNCE_MS = 5;
const int TRANSITIONS_PER_STEP = 4;  // EC11: a full quadrature cycle per detent
const int CLOCKWISE = 1;             // set to -1 if the knob reports the wrong direction

struct Key {
  bool pressed;
  bool reading;
  uint32_t readingSince;
};

Key keys[KEY_COUNT];
uint32_t scannedAt = 0;
uint8_t encoderState = 0;
volatile int encoderSteps = 0;

uint8_t readEncoderPins() {
  return digitalRead(ENCODER_A_PIN) << 1 | digitalRead(ENCODER_B_PIN);
}

void onEncoderChange() {
  // Indexed by previous and current pin state: +1 or -1 for a valid step, 0 for a bounce
  static const int8_t TRANSITION[16] = {0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0};
  static int transitions = 0;

  encoderState = (encoderState << 2 | readEncoderPins()) & 0x0F;
  transitions += TRANSITION[encoderState];
  if (transitions >= TRANSITIONS_PER_STEP) {
    encoderSteps++;
    transitions = 0;
  } else if (transitions <= -TRANSITIONS_PER_STEP) {
    encoderSteps--;
    transitions = 0;
  }
}

InputEvent eventFor(int key, bool pressed) {
  int value = pressed ? 1 : 0;
  if (key == ENCODER_BUTTON_KEY) {
    return {InputType::EncoderButton, 0, value};
  }
  if (key / COLUMNS == ROWS - 1) {
    return {InputType::Button, key % COLUMNS, value};
  }
  return {InputType::Pad, key, value};
}

void debounce(int key, bool reading, uint32_t now, void (*handle)(const InputEvent&)) {
  Key& k = keys[key];
  if (reading != k.reading) {
    k.reading = reading;
    k.readingSince = now;
  } else if (reading != k.pressed && now - k.readingSince >= DEBOUNCE_MS) {
    k.pressed = reading;
    handle(eventFor(key, reading));
  }
}

}  // namespace

void inputBegin() {
  for (int pin : COLUMN_PINS) {
    pinMode(pin, INPUT_PULLUP);
  }
  for (int pin : ROW_PINS) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, HIGH);
  }
  pinMode(ENCODER_BUTTON_PIN, INPUT_PULLUP);
  pinMode(ENCODER_A_PIN, INPUT_PULLUP);
  pinMode(ENCODER_B_PIN, INPUT_PULLUP);
  encoderState = readEncoderPins();
  attachInterrupt(digitalPinToInterrupt(ENCODER_A_PIN), onEncoderChange, CHANGE);
  attachInterrupt(digitalPinToInterrupt(ENCODER_B_PIN), onEncoderChange, CHANGE);
}

void inputScan(void (*handle)(const InputEvent& event)) {
  noInterrupts();
  int steps = encoderSteps;
  encoderSteps = 0;
  interrupts();
  for (; steps > 0; steps--) {
    handle({InputType::Encoder, 0, CLOCKWISE});
  }
  for (; steps < 0; steps++) {
    handle({InputType::Encoder, 0, -CLOCKWISE});
  }

  uint32_t now = millis();
  if (now == scannedAt) {
    return;
  }
  scannedAt = now;

  // A pressed key connects its column to the row driven low, through the key's diode
  for (int row = 0; row < ROWS; row++) {
    digitalWrite(ROW_PINS[row], LOW);
    delayMicroseconds(5);  // let the column lines settle
    for (int column = 0; column < COLUMNS; column++) {
      debounce(row * COLUMNS + column, digitalRead(COLUMN_PINS[column]) == LOW, now, handle);
    }
    digitalWrite(ROW_PINS[row], HIGH);
  }
  debounce(ENCODER_BUTTON_KEY, digitalRead(ENCODER_BUTTON_PIN) == LOW, now, handle);
}
