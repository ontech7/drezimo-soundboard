#pragma once

// GPIO map of the PCB, as in hardware/scripts/design.py
const int COLUMN_PINS[] = {9, 10, 11, 12, 13, 14};
const int ROW_PINS[] = {5, 4, 3, 2, 1, 0, 6};  // the last row is the function keys
const int LED_DATA_PIN = 8;
const int OLED_SDA_PIN = 28;
const int OLED_SCL_PIN = 29;
const int ENCODER_A_PIN = 26;
const int ENCODER_B_PIN = 15;
const int ENCODER_BUTTON_PIN = 27;
const int DEV_LED_PIN = 16;  // the module's own RGB LED
