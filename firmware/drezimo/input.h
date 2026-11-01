#pragma once

const int PAD_COUNT = 36;
const int BUTTON_COUNT = 6;

enum class InputType { Pad, Button, EncoderButton, Encoder };

struct InputEvent {
  InputType type;
  int index;  // pad 0-35 or function key 0-5
  int value;  // 1 pressed, 0 released; for the encoder +1 clockwise, -1 anticlockwise
};

void inputBegin();
void inputScan(void (*handle)(const InputEvent& event));
