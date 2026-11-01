// Drezimo Soundboard firmware for the RP2040-Zero.
// Serial messages are described in docs/project.md, section 9.

#include <Adafruit_NeoPixel.h>
#include <tusb.h>
#include "display.h"
#include "input.h"
#include "leds.h"
#include "logo.h"
#include "pins.h"
#include "settings.h"

const char FIRMWARE_VERSION[] = "0.2.0";
const size_t MAX_LINE_LENGTH = 2100;
const int MAX_FIELDS = 6;
const uint32_t LOGO_FRAME_MS = 25;
const uint8_t AMBER[] = {255, 176, 32};  // the logo's accent colour

struct Command {
  const char* name;
  bool (*run)(char** args, int count);  // false for invalid arguments
};

Adafruit_NeoPixel devLed(1, DEV_LED_PIN, NEO_GRB + NEO_KHZ800);
int devLedMirror = -1;
uint32_t devLedColor = 0;

char line[MAX_LINE_LENGTH + 1];
size_t lineLength = 0;
bool lineTooLong = false;

uint8_t frame[DISPLAY_FRAME_SIZE];
bool appConnected = false;
bool appFrameShown = false;
uint32_t logoShownAt = 0;
uint32_t logoDrawnAt = 0;

// A press that wakes the blank display is swallowed, together with its release
bool swallowed[PAD_COUNT + BUTTON_COUNT + 1];

void showDevLed(uint32_t color) {
  devLedColor = color;
  devLed.setPixelColor(0, color);
  devLed.show();
}

// True if text is a whole number between min and max
bool parseNumber(const char* text, int min, int max, int& value) {
  char extra;
  return sscanf(text, "%d%c", &value, &extra) == 1 && value >= min && value <= max;
}

bool parseColor(char** args, int& red, int& green, int& blue) {
  return parseNumber(args[0], 0, 255, red) && parseNumber(args[1], 0, 255, green) && parseNumber(args[2], 0, 255, blue);
}

bool parseEffect(const char* text, LedEffect& effect) {
  if (strcmp(text, "SOLID") == 0) {
    effect = LedEffect::Solid;
  } else if (strcmp(text, "BREATHE") == 0) {
    effect = LedEffect::Breathe;
  } else if (strcmp(text, "BLINK") == 0) {
    effect = LedEffect::Blink;
  } else {
    return false;
  }
  return true;
}

int hexDigit(char c) {
  if (c >= '0' && c <= '9') {
    return c - '0';
  }
  if (c >= 'A' && c <= 'F') {
    return c - 'A' + 10;
  }
  return -1;
}

bool parseFrame(const char* hex, uint8_t* bytes) {
  if (strlen(hex) != DISPLAY_FRAME_SIZE * 2) {
    return false;
  }
  for (int i = 0; i < DISPLAY_FRAME_SIZE; i++) {
    int high = hexDigit(hex[2 * i]);
    int low = hexDigit(hex[2 * i + 1]);
    if (high < 0 || low < 0) {
      return false;
    }
    bytes[i] = high << 4 | low;
  }
  return true;
}

bool hello(char**, int) {
  Serial.printf("DREZIMO,%s\n", FIRMWARE_VERSION);
  return true;
}

bool setLed(char** args, int count) {
  int index, red, green, blue;
  LedEffect effect = LedEffect::Solid;
  if (count < 4 || count > 5 || !parseNumber(args[0], 0, LED_COUNT - 1, index) || !parseColor(args + 1, red, green, blue)) {
    return false;
  }
  if (count == 5 && !parseEffect(args[4], effect)) {
    return false;
  }
  ledsSet(index, red, green, blue, effect);
  return true;
}

bool setBrightness(char** args, int count) {
  int percent;
  if (count != 1 || !parseNumber(args[0], 0, 100, percent)) {
    return false;
  }
  ledsSetBrightness(percent);
  return true;
}

bool showFrame(char** args, int count) {
  if (count != 1 || !parseFrame(args[0], frame)) {
    return false;
  }
  displayShow(frame);
  appFrameShown = true;
  return true;
}

bool wake(char**, int) {
  displayWake();
  return true;
}

bool changeSetting(char** args, int count) {
  int value;
  if (count != 2 || !parseNumber(args[1], 0, UINT16_MAX, value) || !settingsSet(args[0], value)) {
    return false;
  }
  Serial.println("OK");
  return true;
}

bool sendSettings(char**, int) {
  Serial.printf("SETTINGS,%u,%u,%u,%u\n", settings.oledBrightness, settings.oledDimAfter, settings.oledOffAfter,
                settings.oledShiftEvery);
  return true;
}

bool rebootToBootloader(char**, int) {
  Serial.println("OK");
  Serial.flush();
  delay(100);  // let the reply reach the PC before USB goes away
  rp2040.rebootToBootloader();
  return true;
}

bool setDevLed(char** args, int count) {
  int index, red, green, blue;
  if (count == 2 && strcmp(args[0], "MIRROR") == 0 && parseNumber(args[1], 0, LED_COUNT - 1, index)) {
    devLedMirror = index;
    return true;
  }
  if (count == 3 && parseColor(args, red, green, blue)) {
    devLedMirror = -1;
    showDevLed(devLed.Color(red, green, blue));
    return true;
  }
  return false;
}

const Command COMMANDS[] = {
    {"HELLO", hello},           {"LED", setLed},
    {"BRIGHT", setBrightness},  {"DISPLAY", showFrame},
    {"WAKE", wake},             {"SET", changeSetting},
    {"SETTINGS", sendSettings}, {"BOOT", rebootToBootloader},
    {"DEVLED", setDevLed},
};

// Splits text at the commas, in place. Extra fields stay joined to the last one.
int splitFields(char* text, char* fields[], int maxFields) {
  int count = 0;
  fields[count++] = text;
  while (count < maxFields && (text = strchr(text, ','))) {
    *text++ = '\0';
    fields[count++] = text;
  }
  return count;
}

void handleMessage(char* message) {
  for (char* c = message; *c; c++) {
    *c = toupper(*c);
  }
  char* fields[MAX_FIELDS];
  int count = splitFields(message, fields, MAX_FIELDS);
  if (*fields[0] == '\0') {
    return;
  }

  for (const Command& command : COMMANDS) {
    if (strcmp(fields[0], command.name) == 0) {
      if (!command.run(fields + 1, count - 1)) {
        Serial.println("ERR,invalid arguments");
      }
      return;
    }
  }
  Serial.println("ERR,unknown message");
}

void readMessages() {
  while (Serial.available()) {
    char c = Serial.read();
    if (c == '\n' || c == '\r') {
      if (!lineTooLong) {
        line[lineLength] = '\0';
        handleMessage(line);
      }
      lineLength = 0;
      lineTooLong = false;
    } else if (lineLength < MAX_LINE_LENGTH) {
      line[lineLength++] = c;
    } else {
      lineTooLong = true;
    }
  }
}

void report(const InputEvent& event) {
  if (!appConnected) {
    return;
  }
  switch (event.type) {
    case InputType::Pad:
      Serial.printf("PAD,%d,%d\n", event.index, event.value);
      break;
    case InputType::Button:
      Serial.printf("BTN,%d,%d\n", event.index, event.value);
      break;
    case InputType::EncoderButton:
      Serial.printf("ENCBTN,%d\n", event.value);
      break;
    case InputType::Encoder:
      Serial.printf("ENC,%+d\n", event.value);
      break;
  }
}

int keyNumber(const InputEvent& event) {
  switch (event.type) {
    case InputType::Pad:
      return event.index;
    case InputType::Button:
      return PAD_COUNT + event.index;
    default:
      return PAD_COUNT + BUTTON_COUNT;
  }
}

void onInput(const InputEvent& event) {
  bool wokeDisplay = displayWake();
  if (event.type == InputType::Encoder) {
    if (!wokeDisplay) {
      report(event);
    }
    return;
  }

  bool& swallow = swallowed[keyNumber(event)];
  if (event.value == 1) {
    swallow = wokeDisplay;
  }
  if (!swallow) {
    report(event);
  }
  if (event.value == 0) {
    swallow = false;
  }
}

void updateConnection() {
  bool connected = Serial;  // the app holds the port open (DTR on)
  if (connected == appConnected) {
    return;
  }
  appConnected = connected;
  appFrameShown = false;
  ledsClear();
  if (!connected) {
    logoShownAt = millis();
  }
}

// Until the app sends its first frame the display shows the logo, and the pads show it too
// while no app is connected
void showLogo() {
  uint32_t now = millis();
  if (appFrameShown || now - logoDrawnAt < LOGO_FRAME_MS) {
    return;
  }
  logoDrawnAt = now;
  uint32_t ms = now - logoShownAt;
  drawLogoScreen(frame, ms);
  displayShow(frame);

  if (appConnected) {
    return;
  }
  for (int pad = 0; pad < PAD_COUNT; pad++) {
    bool lit = logoPadLit(pad, ms) && !displayIsOff();
    ledsSet(pad, lit ? AMBER[0] : 0, lit ? AMBER[1] : 0, lit ? AMBER[2] : 0);
  }
}

void updateDevLed() {
  if (devLedMirror >= 0 && ledsShown(devLedMirror) != devLedColor) {
    showDevLed(ledsShown(devLedMirror));
  }
}

void setup() {
  Serial.begin(115200);
  settingsLoad();
  devLed.begin();
  showDevLed(0);
  ledsBegin();
  inputBegin();
  displayBegin();
}

void loop() {
  updateConnection();  // before the messages: a new connection clears the LEDs the app then sends
  readMessages();
  inputScan(onInput);

  bool pcAsleep = tud_suspended();
  displaySuspend(pcAsleep);
  ledsSuspend(pcAsleep);

  showLogo();
  ledsUpdate();
  updateDevLed();
  displayUpdate();
}
