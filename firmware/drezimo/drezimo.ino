// Drezimo Soundboard firmware for the RP2040-Zero.
// Serial messages are described in docs/project.md, section 9.

#include <Adafruit_NeoPixel.h>

const char FIRMWARE_VERSION[] = "0.1.0";
const unsigned MAX_MESSAGE_LENGTH = 64;
const int DEV_LED_PIN = 16;

// The module's own RGB LED. Only useful during development: on the PCB it faces the board.
Adafruit_NeoPixel devLed(1, DEV_LED_PIN, NEO_GRB + NEO_KHZ800);
bool colorCycle = true;
String incoming;

void showDevLed(uint32_t color) {
  devLed.setPixelColor(0, color);
  devLed.show();
}

void handleMessage(String message) {
  message.trim();
  message.toUpperCase();
  if (message.length() == 0) {
    return;
  }

  int r, g, b;
  if (message == "HELLO") {
    Serial.printf("DREZIMO,%s\n", FIRMWARE_VERSION);
  } else if (message == "DEVLED,CYCLE") {
    colorCycle = true;
    Serial.println("OK");
  } else if (sscanf(message.c_str(), "DEVLED,%d,%d,%d", &r, &g, &b) == 3) {
    colorCycle = false;
    showDevLed(devLed.Color(constrain(r, 0, 255), constrain(g, 0, 255), constrain(b, 0, 255)));
    Serial.println("OK");
  } else if (message == "BOOT") {
    Serial.println("OK");
    Serial.flush();
    delay(100);  // let the reply reach the PC before USB goes away
    rp2040.rebootToBootloader();
  } else {
    Serial.println("ERR,unknown message");
  }
}

void setup() {
  Serial.begin(115200);
  devLed.begin();
  devLed.setBrightness(40);  // full brightness is blinding
}

void loop() {
  while (Serial.available()) {
    char c = Serial.read();
    if (c == '\n' || c == '\r') {
      handleMessage(incoming);
      incoming = "";
    } else if (incoming.length() < MAX_MESSAGE_LENGTH) {
      incoming += c;
    }
  }

  if (colorCycle) {
    uint16_t hue = millis() * 16;  // full colour wheel every ~4 s
    showDevLed(devLed.gamma32(devLed.ColorHSV(hue)));
    delay(10);
  }
}
