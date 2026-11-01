#include <EEPROM.h>
#include <string.h>
#include "settings.h"

namespace {

const uint32_t MAGIC = 0xD5E70001;  // change it whenever Settings changes layout
const int MAX_MINUTES = 24 * 60;

struct Stored {
  uint32_t magic;
  Settings settings;
};

}  // namespace

Settings settings = {50, 1, 10, 3};

void settingsLoad() {
  EEPROM.begin(sizeof(Stored));
  Stored stored;
  EEPROM.get(0, stored);
  if (stored.magic == MAGIC) {
    settings = stored.settings;
  }
}

bool settingsSet(const char* name, int value) {
  bool isMinutes = value >= 0 && value <= MAX_MINUTES;
  if (strcmp(name, "OLED_BRIGHTNESS") == 0 && value >= 0 && value <= 100) {
    settings.oledBrightness = value;
  } else if (strcmp(name, "OLED_DIM_AFTER") == 0 && isMinutes) {
    settings.oledDimAfter = value;
  } else if (strcmp(name, "OLED_OFF_AFTER") == 0 && isMinutes) {
    settings.oledOffAfter = value;
  } else if (strcmp(name, "OLED_SHIFT") == 0 && isMinutes) {
    settings.oledShiftEvery = value;
  } else {
    return false;
  }

  EEPROM.put(0, Stored{MAGIC, settings});
  EEPROM.commit();
  return true;
}
