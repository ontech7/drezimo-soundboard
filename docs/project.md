# DIY physical soundboard for Voicemod

> Note: the project is in English. The OLED display texts can be localised: English and Italian so far: the system's language by default, or the one set in the app.

> Project document  
> Status: PCB v0.2 ordered with its LEDs fab-assembled; firmware 0.2.0 tested on a bare RP2040-Zero; the bridge app drives Voicemod, the display, the PC overlay and the board's firmware updates, and installs and updates itself on Windows. Next: the assembled PCB and the enclosure.

**Changelog**
- 2026-10-29: moved to 6×6 + custom PCB + USB-only power.
- 2026-11-01: parts measured; PCB moved to the RP2040-Zero (USB-C through the back edge), LED cut-outs at the measured 4.47 mm, OLED header VCC-GND-SCL-SDA.
- 2026-11-01: 1:1 paper fit check passed; Kailh hot-swap sockets (fab assembled) instead of soldered switches; PCB fully routed with GND pours on both layers; 5 mm PCB corners.
- 2026-11-01: JLCPCB fab files (`export_fab.py`); LCSC parts chosen, 47 µF moved to a 1206 Basic part; the LED needs Standard PCBA.
- 2026-11-01: firmware bring-up on a bare RP2040-Zero (`firmware/`): USB serial messages, module LED, reboot to the bootloader and upload with one script.
- 2026-11-01: hot-swap sockets dropped (out of stock at JLCPCB): switches soldered to the PCB in plated holes, board v0.2. The 5 bare v0.1 boards of the first order are hot-swap only (NPTH pin holes).
- 2026-11-01: firmware 0.2.0 implements the board ↔ app protocol (section 9): keys and encoder, LED effects with the power limiter, app-drawn display frames, burn-in protection, saved settings, and an animated logo on the display and the pads while no app is connected, also in the simulator.
- 2026-11-01: the simulator drives a real board over Web Serial (Chrome/Edge), as the bridge app will.
- 2026-11-01: bridge app started with Tauri 2 (Rust core, measured against Electron): tray icon, automatic board search, handshake.
- 2026-11-01: the app drives Voicemod: pads play the sounds of the current page, function keys and knob work, LEDs show the sound states (playback estimated until the log watcher).
- 2026-11-01: the app follows Voicemod's log: real playback state and learned sound lengths, also for sounds played from Voicemod's window.
- 2026-11-01: the app draws the board's display, pixel for pixel as the simulator (also previewed in the app's window), with the soundboard menu, peek and row overview on the knob.
- 2026-11-02: Windows installer published by a release workflow; the app starts at login and updates itself.
- 2026-11-02: the app updates the board's firmware by itself when the board has an older one.
- 2026-11-02: the app shows the 6×6 PC overlay (sound names and live LED colours) when the page changes and while the knob is held, in a native window; the simulator previews it the same way.

## 1. Goal

Build a custom, inexpensive, 3D-printed physical soundboard to use with Voicemod on a PC.

The device will have:
- **36 sound pads** in a 6×6 grid to trigger sounds.
- **A row of 6 function keys**: PREV page, NEXT page, STOP, MUTE ME, VOICE, HEAR ME.
- **An EC11 rotary encoder with push button**: rotate = page, tap = soundboard menu, hold = identify sounds.
- **42 addressable RGB LEDs** (SK6812 MINI-E, reverse-mount), one under each pad and function key, to show sound availability and status.
- **A 1.3" OLED display** (128×64, SH1106, I²C) showing the page, the last sound and the playback/device status.
- **A custom 2-layer PCB** carrying the key matrix, the LEDs and all the electronics.
- **A single USB cable** to the PC for both data and power: no second power supply.
- **A desktop bridge app** on the PC (Windows, also macOS) that connects the physical controls to Voicemod.

The project favours readily available components, a repairable structure and the ability to extend it in the future. Whoever uses it must never need to touch the board: bug fixes ship as bridge-app updates.

### Quality bar: a sellable product

The device must look and feel like a finished commercial product, not a prototype. Every part of the project is held to this bar; "it works" is not enough if it looks or behaves like a hobby build.

**Cost target:** the total parts cost of one unit (PCB and assembly, components, switches, keycaps, enclosure filament, cable) must stay **at or below €50**, otherwise the product makes no sense. Quality comes from design and software, not from expensive parts: every component choice is checked against this target, prefers the fab's basic parts and bulk sources, and avoids paid options that do not change how the product looks or works.

- **Display UI:** a designed interface, not debug text. Consistent layout grid with a 1 px safe margin, one custom pixel font with full Italian accents (à è é ì ò ù), small icons for states (mic off, loop, pause, mute) instead of words where possible, a remaining-time bar, smooth scrolling for long names, short page transitions, double-buffered drawing (no flicker), a boot splash, and clear, actionable error screens (e.g. "Open Voicemod" instead of "WS error").
- **Display care:** brightness control, dim and blank on idle, pixel shift (see "Burn-in protection").
- **LEDs:** gamma-corrected colours balanced for equal perceived brightness, short fades (≈100–200 ms) instead of hard on/off, a subtle power-on animation, and a power limiter that dims smoothly with no visible steps.
- **Enclosure:** ergonomic and soft-edged, but easy to print on a P1S without supports:
  - rounded visible edges (fillets); bed-facing edges chamfered so they print cleanly;
  - a slight typing angle built into the shape;
  - no screws visible from the top: M3 screws enter from the bottom;
  - seams placed at the back, consistent wall thickness, no support marks on visible faces;
  - consider printing the top face down on the textured PEI plate for an even matte texture;
  - tolerances proven with small test prints before the full print.
- **Connections:** the USB-C cable exits cleanly from the top (back) edge with strain relief; the knob and keycaps sit straight and aligned.
- **PCB:** colour is irrelevant (cheapest standard option); the bottom silkscreen carries the logo (with the project name) and the hardware version.
- **Logo:** "Pad Grid": the 6×6 pads, with the lit ones drawing a D, next to a custom DREZIMO / SOUNDBOARD wordmark; amber (`#FFB020`) is the only accent colour. Every version shares the same geometry (`assets/logo/`): SVGs for dark and light backgrounds, a single-colour version, the app icon, the OLED boot screen (pixel exact, 4 px margin so pixel shift never clips it) and the KiCad silkscreen footprint. Voicemod's name and mark are deliberately not part of it (trademark).
- **Desktop app:** one-click installer, auto-start, tray icon, silent auto-update with firmware updates and a progress bar, and a polished settings window. The installers are not code-signed, by choice (no yearly certificate costs): Windows asks to confirm an unknown publisher at the first install (More info → Run anyway), and on macOS the app is opened once with "Open Anyway" or after `xattr -cr "/Applications/Drezimo Soundboard.app"`.

## 2. General architecture

The system is split into three parts:

1. **Hardware controller — RP2040-Zero (RP2040, USB-C)**
   - Scans the key matrix (36 pads + 6 function keys) and reads the encoder.
   - Drives the 42 LEDs and the OLED display, following updates from the bridge app.
   - Communicates with the PC over USB serial.
   - Enforces the LED power limit in firmware (see section 6).
   - Can be rebooted into its USB bootloader by a command from the app, so firmware updates do not need physical BOOTSEL access.
   - A clone of the Waveshare RP2040-Zero replaces the Raspberry Pi Pico: smaller, with USB-C, mounted under the PCB so the cable leaves from the back edge.

2. **Bridge app on the PC (Windows)**
   - Manages pages (36 sounds each, in Voicemod's soundboard order, plus a Favourites page).
   - Receives events from the board and sends commands to Voicemod through the official Control API.
   - Includes the Voicemod log watcher, which provides the real playback state (see section 8).
   - Sends pad colours and display contents to the board.
   - Shows a 6×6 overlay on the PC screen with sound names and live LED colours.
   - Updates the board firmware when needed.
   - Installed once with an installer and started automatically with Windows.

3. **Voicemod**
   - Plays sounds; all playback semantics stay inside Voicemod (see "Voicemod soundboard behaviour (verified)").
   - Integration verified with the official Control API and an official client key; the keyboard-shortcut fallback is no longer needed.

### Expected flow

`Pad / button / encoder → RP2040-Zero → USB → bridge app → Voicemod`

For visual updates:

`Voicemod (Control API + log) → bridge app → USB → RP2040-Zero → LEDs + OLED`

## 3. Control mapping

### Main pads

- **36 pads**, arranged in a 6×6 grid.
- Each page shows 36 sounds, following Voicemod's soundboard order; a **Favourites** page collects the `favorited` sounds.
- A pad press is exactly a click on the sound in Voicemod.
- A pad without a sound stays unlit; sounds not included in the licence are shown in dim red and not sent.

### Function keys

A row of **6 function keys** above/below the grid:

| Key | Action | Voicemod Control API | LED |
|---|---|---|---|
| PREV | Previous page | — (handled by the app) | Dim white when more pages exist |
| NEXT | Next page | — (handled by the app) | Dim white when more pages exist |
| STOP | Stops everything, paused sounds included | `stopAllMemeSounds` | Red while any sound plays |
| MUTE ME | "Mute for me" on/off | `toggleMuteMemeForMe` | Amber when on |
| VOICE | Voice changer on/off | `toggleVoiceChanger` | Green when on |
| HEAR ME | "Hear myself" on/off | `toggleHearMyVoice` | Green when on |

There is no per-sound stop in the API: a single sound is stopped through its own playback mode, as in Voicemod.

### EC11 encoder

Incremental encoder with continuous rotation and integrated push button (3 GPIOs, 3.3 V logic):
- **rotate** = change page;
- **tap** = soundboard menu (choose the active soundboard);
- **hold + press a pad** = peek (see below);
- **hold + rotate** = row overview (see below).

### Identifying sounds without key displays

The keycaps have no legends or screens, so sounds are identified in other ways:

1. **Peek:** hold the encoder button and press a pad → the display shows the sound's name, the pad flashes, no sound is played.
2. **Row overview:** hold the encoder button and rotate → one row of 6 pads lights white and the display lists their names.
3. **PC overlay:** the bridge app shows a 6×6 grid on the PC screen with the names and live LED colours, for a few seconds when the page changes and while the knob is held.
4. **Later:** custom per-sound idle colours set in the app.

Points 1–3 are already implemented in `simulator/dummy-board.html` (peek also via right-click on a pad; the overlay is previewed next to the zoomed display).

## 4. Expected LED and display behaviour

The colours are a project convention implemented in software, based on the state Voicemod actually exposes (Control API + log watcher).

### Pad LEDs

| Pad state | Colour | Meaning |
|---|---|---|
| Empty pad | Off | No sound on this pad |
| Sound available | Blue | The pad has a playable sound |
| Pad just pressed | Yellow, ~200 ms | Immediate press feedback |
| Sound playing | Green | Predicted on press, confirmed by the log watcher |
| Sound paused | Breathing green | `PlayPause` sound paused; still active in Voicemod |
| Not in licence | Dim red | Sound locked by the licence; not sent |
| Voicemod unreachable | Blinking red | Control API connection lost or failed |

### Function-key LEDs

Function-key LEDs mirror Voicemod's toggles: STOP red while any sound plays, MUTE ME amber when on, VOICE and HEAR ME green when on, PREV/NEXT dim white when more pages exist.

### LED chain and light path

- LED chain index: **0–35** pads (row by row), **36–41** function keys.
- The LEDs are reverse-mount SK6812 MINI-E on the PCB bottom, shining up through a cutout into the switch's LED window.
- The keycaps are opaque OEM double-shot PBT: the light shows as a **halo** around each key (accepted by the author).

### State priority and duration

- The yellow feedback lasts about 200 ms; duration to be tuned.
- Green is applied immediately on press as a prediction and confirmed (or corrected) by the log watcher; loop sounds stay green until stopped.
- When the sound ends, the pad returns to blue.
- Total LED current is capped by the firmware power limiter (section 6): with many LEDs lit at once, brightness is scaled down.
- Colours must be configurable at a later stage.

### Display (OLED)

- 1.3" OLED, 128×64, SH1106 driver, I²C, 4 pins. Module 35.54 × 33.59 mm, glass 34.44 × 22.97 mm, pixel area ≈29.71 × 14.90 mm (measured).
- **The pin order differs between modules** (VCC-GND-SCL-SDA vs GND-VCC-SCL-SDA). The modules used here are VCC-GND-SCL-SDA seen from the glass side, and the PCB follows them: check the silkscreen of any replacement module.
- Contents: page name + counter, last sound, remaining time / `LOOP` / `PAUSED` / `MIC OFF` / `MUTE ME` / `VOICE OFF`; sound names for peek and row overview.
- Texts are localisable: English and Italian so far, the system's language unless another is set in the app.
- OLED pixels emit their own light (no backlight): perfectly readable in the dark, weakest in direct sunlight.

#### Burn-in protection (firmware)

OLED pixels age with use, and the header text stays in the same place for hours: without protection it leaves a permanent shadow. The firmware applies these rules, in this order of escalation:

| Rule | Behaviour | Default | Adjustable |
|---|---|---|---|
| Brightness | Normal display brightness (SH1106 contrast register) | Medium | Yes, 0–100 % |
| Dim when idle | After N minutes without any key, knob or Voicemod event, drop to low brightness | 1 min | Yes, or off |
| Blank when idle | After N minutes idle, turn the panel off (SH1106 display-off command, not just black pixels) | 10 min | Yes, or off |
| Pixel shift | Move the whole image by 1 px every N minutes, cycling through a small pattern (e.g. ±1 px in x and y) so no pixel stays lit in the same spot | On, every 3 min | Yes, on/off and interval |

- **Wake-up:** any key, knob turn or knob press restores full brightness immediately. The input that wakes a blanked display is consumed (it does not play a sound), so the user can wake it safely; a sound started from the PC also wakes it.
- **Layout margin:** the UI is drawn with a 1 px border free on every side, so the pixel shift never clips text.
- **Settings:** changed from the desktop app and stored in the board's flash, so they survive a PC change or an app reinstall. The values in the table are only defaults.

## 5. Components

### Bill of materials

| Component | Quantity | Notes |
|---|---|---|
| RP2040-Zero (Waveshare, or a compatible clone) | 1 | USB-C; mounted on its pin strips under the PCB |
| MX-compatible switches, 5-pin | 42 | Akko V5 linear here, with a clear top housing for the LED |
| Keycaps, OEM profile, double-shot PBT, opaque | 42 | Halo lighting around the caps |
| EC11 rotary encoder with push button, and its knob | 1 | Rotation and push button |
| SN74AHCT125N, DIP-14 | 1 | 3.3 V → 5 V level shifting of the LED data signal |
| 1.3" OLED 128×64, SH1106, I²C | 1 | Check its pin order (section 4) |
| M3 × 10 mm screws | 6 | Closing the enclosure |
| Rubber feet | a few | Enclosure bottom; the count follows the enclosure's design |
| USB-C cable | 1 | Data + power to the PC (C-to-C or A-to-C) |

Tools: a 3D printer for the enclosure (designed on a Bambu Lab P1S, matte PLA or PETG) and a soldering iron.

### Comes with the PCB order (assembled by the fab, PCB bottom side)

| Component | Quantity | LCSC | Use |
|---|---|---|---|
| SK6812MINI-E reverse-mount RGB LED (OPSCO) | 42 | C5149201 (Extended) | One per pad and function key |
| 100 nF 50 V X7R 0603 | 43 | C14663 (Basic) | One per LED, plus one for the SN74AHCT125N |
| 1N4148W, SOD-123 | 42 | C81598 (Basic) | Key matrix (anti-ghosting) |
| 47 µF 10 V X5R 1206 | 1 | C96123 (Basic) | 5 V LED rail, after the polyfuse |
| 330 Ω 1 % 0603 | 1 | C23138 (Basic) | In series with the LED data line |
| PTC 0.5 A hold / 1 A trip, 1206 | 1 | C720075 (Extended) | Protection of the USB 5 V rail |

Stock and Basic/Extended class checked on 2026-11-01.

### Measurements of the real parts (2026-11-01)

- [x] Akko V5 LED window: 6.00 × 2.04 mm, its centre 4.47 mm from the switch centre (not the 5.08 mm MX default); the LED cut-outs are placed there.
- [x] OLED module: 35.54 × 33.59 mm, pins VCC-GND-SCL-SDA, pin 1 centre 13.92 mm from the left edge and 1.69 mm from the top edge, straight header already soldered (2.5 mm spacer), 3.02 mm glass + board, 4.07 mm overall. Four Ø3.2 mm holes: top pair 2.64 / 3.03 mm from the side / top edges, bottom pair closer to the bottom edge (≈2.4 / ≈2.0 mm, from the photo: measure before designing the enclosure).
- [x] EC11: matches the stock Alps EC11E footprint (A-C-B 2.5 mm pitch, switch pins 5 mm, rows ≈14.5 mm apart, mounting lugs ≈11.4 mm apart); 12 × 12 × 6.45 mm body, M7 threaded bushing with nut, 6 mm knurled shaft, 20 mm above the body. Knob Ø14.4 × 17 mm.
- [x] RP2040-Zero: 23.30 × 17.90 × 1.04 mm, 23 edge pins on the 2.54 mm grid (columns 15.24 mm apart, first row 1.43 mm from the USB edge, 5-pin row in line with the last pins of the columns), USB-C on the face opposite the chip, overhanging the board by 1.23 mm. The 5V pin is VBUS directly (5.02 V, no diode). Enumerates as RPI-RP2 with both C-to-C and A-to-C cables (the CC resistors are fitted).

## 6. Electronics, PCB and power

### Custom PCB

- 2-layer PCB from JLCPCB or similar; about 35–70 € for 5 boards including shipping (indicative).
- **Fab files** (`hardware/scripts/export_fab.py`, into `hardware/fab/`): Gerber + Excellon zip (PTH and NPTH in separate files), assembly BOM with the LCSC part numbers (read from each symbol's `LCSC` field) and the placement file (CPL). Gerbers, drill and CPL share the drill/place origin. CPL positions are the centre of each part's pads, and bottom rotations follow JLCPCB's convention, checked against each part's JLCPCB footprint.
- **Assembly at JLCPCB:** bottom side only, **Standard PCBA**: the SK6812MINI-E is a "Standard only" part (≈ $34 of fixed setup and stencil fees per order, Oct 2026, plus $3 for each Extended part line). In the placement preview check that each LED's pin-1 dot is on the chamfered corner of its silkscreen outline and each diode's cathode on the silkscreen bar.
- **SMD parts assembled by the fab** on the PCB bottom side: LEDs, 100 nF per LED, matrix diodes, 47 µF bulk capacitor, 100 nF for the buffer, 330 Ω data resistor, 0.5 A polyfuse on the LED rail.
- **Hand-soldered by the author:**
  - The 42 switches, from the bottom.
  - RP2040-Zero on its 2.54 mm pin strips, under the PCB with the USB-C face down and the chip side towards the board, USB-C at the back edge. Trim the pin ends on the top side to 2 mm at most.
  - EC11 encoder.
  - SN74AHCT125N in its DIP-14 footprint.
  - OLED module, soldered by its own header.
- **Key matrix:** 7 rows × 6 columns (6 pad rows + the function row), one diode per key → 13 GPIOs.
- **Switches:** soldered to the PCB in 1.5 mm plated holes (the Kailh hot-swap socket was out of stock at JLCPCB), and held by the case plate too. The footprint (`hardware/lib/drezimo.pretty/SW_MX_PCB`) has the peg holes of 5-pin switches like the Akko V5. The LED window is in the bottom housing on the side opposite the metal pins: 6.00 × 2.04 mm, centred 4.47 mm from the switch centre (measured), which is where the LED cut-outs are.
- **Board layout** (generated by `hardware/scripts/build_pcb.py`, case frame = origin at the case's top-left outer corner):
  - Case 130.3 × 196.35 mm; PCB 124.3 × 190.35 mm (3 mm inset: 2.5 mm wall + 0.5 mm clearance), 5 mm corner radius (the case can round its corners up to ≈8 mm).
  - Top strip (y 8–46 mm): RP2040-Zero top-left, hanging under the board on its pin strips, USB-C through the back wall (connector centre x = 30 mm, receptacle face 2.27 mm from the case's outer edge); 1.3" OLED module right of it, soldered directly by its header (header centre x = 77.7 mm); EC11 above the last column (shaft at 112.775, 27).
  - GPIO map, chosen so that no two routes cross: ROW0–5 = GP5–GP0 and ROW6 (function keys) = GP6 on the module's left column, fed by the ROW bus coming up the left edge; COL0–5 = GP9–GP14 on its 5-pin row, facing the keys; I²C0 SDA/SCL = GP28/GP29 and encoder A/B/switch = GP26/GP15/GP27 on its right column, facing the OLED and the knob; LED data = GP8; GP7 spare.
  - SN74AHCT125N upright below-left of the module; gate 3 (pins 8–10) drives the LEDs because it sits next to the first LED of the chain (function key 1).
  - LED chain: every row left to right (function row, then pad rows 0–5); the end of each row returns to the start of the next one.
  - Bottom side (fab assembly): switch pins north of each switch centre, LED 4.47 mm south, diode west of pin 2, 100 nF east next to the LED's VDD; polyfuse and 47 µF on the 5 V trunk that runs down the right of the module.
  - 6 M3 holes 6.5 mm from the case edges: 4 corners + middle of the long sides (between pad rows 3 and 4).
  - Review documents in `hardware/docs/` (`export_docs.py`): schematic PDF, 1:1 A4 fit-check PDFs (top and mirrored bottom, with a 100 mm scale bar), 3D renders, DRC report. `hardware/pcba-bottom.png` is a render of the assembled bottom side.
  - Routing (generated too): F.Cu carries the COL and +5V_LED columns and the top-strip signals, B.Cu the ROW trunks and bus and the LED chain; both layers are GND pours, stitched by two vias per key. 0.25 mm signals (0.2 mm for the ROW bus squeezed past the middle mounting hole), 0.4–0.8 mm for +5V_LED.
  - Status: 1:1 paper fit check passed with the real parts (2026-11-01); fully routed, DRC clean, schematic parity 0, no unrouted connections.
- **Level shifter:** the SN74AHCT125N enable pin is active low and must be tied to the correct state; unused inputs must not be left floating. The 330 Ω resistor goes in series with the data line, near the buffer output or the first LED.

### GPIO budget

| Function | GPIOs |
|---|---|
| Key matrix (7 rows + 6 columns) | 13 |
| EC11 encoder (A, B, push button) | 3 |
| LED data | 1 |
| I²C to the OLED (SDA, SCL) | 2 |
| **Total** | **19 of the RP2040-Zero's 20 edge GPIOs** |

### Power: USB only

- A single USB cable from the PC powers everything; there is no second power supply.
- The LEDs are powered from USB 5 V (VBUS, the RP2040-Zero's 5V pin), never from its **3V3** pin.
- USB 2.0 guarantees 500 mA. The RP2040-Zero + OLED draw about 60 mA, so the **firmware power limiter caps the total LED current at ≈400 mA**, scaling brightness down when needed.
- Single-colour states (blue, green) draw about 1/3 of white, so the limiter only dims the LEDs when many of them are lit at once.
- Large bulk capacitance on VBUS is avoided (inrush current at plug-in): 47 µF after the polyfuse instead of 1000 µF, plus 100 nF next to each LED.

### Power consumption estimate

A single LED at full white draws in the order of tens of mA (check the SK6812 MINI-E datasheet): 42 LEDs at full white would far exceed what USB can supply, which is why the firmware limiter is mandatory and must be active from the first power-up. Normal use (a few green pads over a blue page) stays well below the cap.

## 7. Physical structure and 3D enclosure

- Enclosure 130.3 × 196.35 mm (key pitch 19.05 mm, 38 mm top strip for the RP2040-Zero, OLED and knob), printed in one piece on a Bambu Lab P1S (256 × 256 × 256 mm build volume), in matte black PLA (or PETG).
- The plate holds the switches; the PCB sits underneath; the bottom is removable for repairs.
- Closed with 6 M3 × 10 mm screws directly into the plastic (≈2.5 mm pilot holes, thick bosses): corners + middle of the long sides, because the wide plate flexes. Hole centres 6.5 mm from the case edges, matching the PCB. Screws go in from the bottom, so none is visible from the top.
- Rubber feet on the bottom.
- Window for the OLED (pixel area ≈29.71 × 14.90 mm) and a hole for the EC11 knob. The module's four holes can locate it from above with pins in the case.
- The Drezimo Soundboard logo raised on the top strip, in the free space left of the OLED window: 42 mm wide, its left edge in line with the left edge of the first column of keys, centred on the window's height, from the shared logo geometry. The function key labels are raised too. Logo and labels are white: the print pauses for a filament change (black to matte white) at the layer where the relief starts, which the P1S does without an AMS. As previewed in the simulator.
- USB-C of the RP2040-Zero exits from the top (back) edge, with strain relief for the cable. The receptacle face is 2.27 mm inside the case's outer edge, centred at x = 30 mm and about 5 mm below the PCB's underside (2.5 mm strip spacer + 1.04 mm module + half the 3.03 mm receptacle): the back wall needs an opening for the plug's overmould, and the bottom must leave at least 7.5 mm under the PCB there (the module hangs 6.6 mm).
- Ergonomics and finish follow the quality bar in section 1: rounded edges, typing angle, support-free printing.
- Do not finalise the 3D model before measuring the finished PCB: the OLED pixel-area position and its bottom pair of holes are still photo estimates.

## 8. Desktop software

The bridge app will have at least these responsibilities:

1. Open the USB serial connection to the board.
2. Receive pad, function key and encoder events.
3. Map each pad to the current page and sound (36 per page, Voicemod's order, Favourites page).
4. Send the commands to Voicemod.
5. Update the LEDs and the display based on the page, press feedback and confirmed states.
6. Watch the Voicemod log for the real playback state.
7. Show connection status, active page, last sound and any errors, plus the 6×6 overlay on the PC screen.
8. Manage an editable configuration without reflashing the board; update its firmware when needed (reboot to USB bootloader by command).
9. Ship as a Windows installer with auto-start; bug fixes are delivered as app updates.

### Implementation: `app/` (Tauri 2, logic in Rust)

A tray app for macOS and Windows, built with Tauri 2. All the logic runs in the Rust core process, which keeps working with no window open; the window (settings, status) is a small React page that exists only while it is open. Chosen over Electron after measuring both on macOS (minimal apps, release builds, memory summed over every process of the app):

| | Electron 45 | Tauri 2.12 |
|---|---|---|
| App size | 252 MB | 3.9 MB |
| Compressed installer | 124 MB | 2.2 MB |
| Memory, window open | 92 MB | 64 MB |
| Memory, no window (the tray case) | 39 MB | 35 MB |

Electron would have reused the simulator's JavaScript as is; Tauri means rewriting that logic in Rust, accepted in exchange for a small, self-contained app. Two constraints shape the design:
- On Windows a hidden Tauri webview is throttled and may be suspended after a few minutes, and Tauri cannot turn that off there. Nothing that must keep running lives in a webview.
- macOS's webview has no Web Serial: the board link uses the `serialport` crate (tested on the bare module: found by its USB ID, DTR, handshake).

Board link (`app/src-tauri/src/board.rs`): every second the app lists the serial ports with the board's USB ID (2E8A:F00A) and opens each one with DTR on; a port that answers `HELLO` with `DREZIMO,<version>` is the board. The ID is arduino-pico's generic one, so a port that leaves 3 handshakes unanswered (3 s apart) is probably another device and is then tried only every 30 s, in case it was a board whose answer got lost; a dedicated product ID would make the filter exact. The handshake starts with a newline, which ends any half line a broken session left in the board's buffer. A port that is listed but cannot be opened is held by another program: it is reported as *in use* and retried every 3 s. One thread reads and writes: Windows runs a serial port's reads and writes one at a time, so reads wait at most 25 ms and messages for the board go out between them (replies at once); writes may wait up to 1 s for room in the USB buffers. An unplugged board is noticed from the read errors or, where a driver reports none, within 2 s from asking the port for its state.

Settings window (`app/src-tauri/src/window.rs`): built on demand, hidden with the app's dark background until the page reports its first complete frame (about 170 ms on an M-series Mac, release build), so it never flashes white. A sidebar leads to four pages: Status (the board and Voicemod; the current page as the board's 6×6 pads, with their names and live LED colours, playable from the window as from the board, held clicks included, with the page picked from a menu without showing the overlay; STOP, MUTE ME, VOICE and HEAR ME, lit as on the board; the display's picture; the latest key presses: all of it works without the board, with Voicemod alone), General (the language, starting with Windows, the versions), Board (the LEDs' brightness, shown live while the slider moves, and the display's settings, read from the board and saved there with `SET` when a control is let go) and Overlay (on or off, one of nine places, size and opacity shown live while their sliders move, duration and a preview). The app's own settings are a JSON file in its config folder (`settings.rs`, `%APPDATA%\dev.drezimo.soundboard\settings.json` on Windows); a missing field takes its default, so older files still load. Starting with Windows works as the switch in Windows' startup apps: turned off, the login item stays but is marked off, so an update, which writes it again, keeps it off; turned on, it clears that mark. It is available only in the installed app. A closed window is only hidden for 5 minutes, for an instant reopen, then destroyed to give its memory back. The app opens it when started by hand and stays in the tray when started with `--autostart`, as the login item does. Each window gets a new label, so one being destroyed never clashes with a new one.

Voicemod link (`app/src-tauri/src/voicemod.rs`): the Control API WebSocket, registered with the client key, which is built into the app from `VOICEMOD_CLIENT_KEY` (environment or the repository's `.env`, never in git; a build without it shows the missing key). After registering, the app asks for the soundboards and the three switches and follows Voicemod's own messages from then on (soundboards edited, soundboard picked in Voicemod, switches changed). When Voicemod is not running the app retries every 3 s, every 60 s after a rejected key; requests made meanwhile are dropped, and the LEDs blink red.

Device logic (`controller.rs`, run by `device.rs` on its own thread): the simulator's behaviour, written as plain logic with unit tests. Pages come from `soundboard.rs` (Favourites first, then every soundboard in Voicemod's order, 36 sounds per page). The current page is kept when the soundboards change (a soundboard that shrank keeps its last page) and when Voicemod comes back after a restart. A pad sends `playMeme` key-down; the key-up goes only to `PlayLoopOnPress` sounds, at least 200 ms after the press, also when the board is unplugged while the key is held. It never goes to a sound stopped meanwhile (STOP, a sound that stops the others), because Voicemod would take it as a press and start the loop again; a pad pressed again before its key-up went out simply counts as still held. `playback.rs` replays Voicemod's per-sound rules to predict what is playing, and the log confirms or corrects it (below). Only the LEDs that change are sent, and all of them when the board connects.

Log watcher (`voicemod_log.rs`): follows `voicemod-desktop.log` (macOS: `~/Library/Application Support/VoicemodV3/logs/`; Windows: `%LOCALAPPDATA%\VoicemodV3\logs\`) every 100 ms, reopening it for each read so Voicemod can rotate it, and going on from the start of a rotated file (a file missing for less than 2 s is taken as a rotation). At start it reads the last 4 MB: the uninterrupted plays there teach the sounds' lengths, and the lines since the last `------ RUN ------` (Voicemod's start) tell what is playing now. A sound whose file is plain Ogg (Vorbis or Opus), as the Community's are, gets its length from the file the line names, read once, until a whole play measures it: so its countdown shows from its first play. The soundboards' files are obfuscated and only the log teaches their lengths; so does the simulator, fed by the Python bridge, for every sound. The lines are replayed through Voicemod's per-sound rules, as `tools/voicemod_log_bridge.py` did, and matched by timestamp with the predictions of the presses (local times, converted with the system time zone): a line within 600 ms of a prediction confirms it, a line nobody predicted (a sound played from Voicemod's window, the real end of a sound) updates the state unless a press changed that sound later, and a prediction left unconfirmed for 4 s makes the app take the state from the log again. A `------ RUN ------` line (Voicemod restarted) clears everything. After 3 predictions in a row left unconfirmed, the log is taken as unreadable (a Voicemod update may change its format) until the app restarts. Without the log, the app guesses 3 s per sound and the window says so. Learned lengths are not saved yet: the log's own history covers them.

Display (`display/`, `controller/screen.rs`): the app draws the 128×64 frames with the simulator's screens and font, Silkscreen at 8 and 16 px (SIL Open Font License, `display/OFL.txt`). Its glyphs were captured from the simulator's own canvas, so the app's frames match the simulator's pixel for pixel (checked on every screen); a star was added for Favourites. The texts follow the language set in the window, the system's by default (Italian or English), as do the window, the overlay and the tray menu. A frame goes to the board only when the picture changes, and to the window, which previews the display. A sound started from the computer sends `WAKE`, so a dimmed display brightens; one outside the soundboards, played from Voicemod's Community, has no name the app can know and shows as "Community". The bottom right shows the latest sound's countdown, `PAUSED` or `LOOP`; when no sound plays, or its end is not known yet, it shows the first of Voicemod's switches away from usual, in this order: `MUTE ME`, `VOICE OFF`, `HEAR ME`. The knob works as in the simulator: a tap opens the soundboard menu (a turn moves, another tap picks, it closes after 6 s); held with a pad press it peeks at the sound (name, mode, length, no sound played; the pad blinks); held and turned it shows a row of six (the row lights white, the others dim).

PC overlay (`overlay/`, `controller/overlay.rs`): the current page as a 6×6 grid, each pad with its sound's name and its live LED colour, under the page's name and number. Blinking and breathing LEDs animate at the board's pace, though not in step with it: the firmware times its effects from its own start. It appears for 4 s when the page changes (knob, PREV/NEXT, menu, a soundboard picked in Voicemod) or the board connects, and stays while the knob is held or the menu is open, so holding the knob also shows it; it fades in and out, and hides while the board or Voicemod is missing. The Rust core draws it itself, in Inter (SIL Open Font License, `overlay/OFL.txt`; subset to Latin with its kerning kept, names on two lines at most), into a native window rather than a webview, so it never sleeps and appears at once: on Windows a layered window with per-pixel transparency that lets the mouse through, never takes the focus and stays out of the taskbar and Alt+Tab; on macOS a borderless window on every Space. It sits on the main screen, clear of the taskbar or the Dock: in the bottom-right corner unless another of nine places is set in the window, which also sets its size (70–100 %: smaller, the names are too small on a screen at 100 %) and opacity, shown live while their sliders move, and its duration, or turns it off; another monitor and dragging it are not done yet. It shows over windowed and borderless games and over most "fullscreen" games on Windows 10/11; it does not inject into game processes, like Discord's overlay does, because anti-cheat systems could flag that. Tested on macOS, and on a Windows desktop (a GitHub Actions runner, reading the screen back): the transparency, the half-opacity fade, the click-through window style and the focus staying where it was; still to be seen over games on a Windows PC.

Firmware updates (`firmware.rs`): the app carries the board's firmware, built into it from `app/src-tauri/firmware/drezimo-<version>.uf2` (written by `firmware/build.sh --bundle`; the build stops if that version differs from the firmware's source). When a board connects with an older version, the app sends `BOOT`, waits for the RP2040's bootloader drive (RPI-RP2) that appears next, writes the UF2 file there and waits for the board to come back with the new version, in about 5 s; the window and the tray say so meanwhile. Only a drive that was not there before `BOOT` is written, so another RP2040 board waiting in its bootloader is left alone. A board with the same or a newer version is never touched. A board that comes back with its old firmware, for whatever reason, is not tried again until the app restarts, so it never loops between its bootloader and the app; it keeps working, and the window shows why the update failed. If the bootloader's drive never appears (removable drives blocked on that PC, say), the board waits in its bootloader and the window asks to unplug it and plug it back in, which starts its old firmware again. The bootloader is in the chip's ROM, and the copy takes about a second: a board unplugged before it starts keeps its old firmware, and is updated at the next start of the app; one unplugged in the middle of it can be left without a working firmware, and then needs the BOOT button inside the case and a new copy. On Windows the drive is found among the removable drives, without the "insert a disk" box for an empty card reader; on macOS in `/Volumes` (copying onto the drive by hand can hang there, the app's single write did not in testing).

Installer, start at login and updates (`updates.rs`, `windows/`, `.github/workflows/release.yml`): the Release workflow, run by hand from `main`, builds the Windows installer (NSIS, installed for the current user, no administrator rights) with the Voicemod key and publishes it as a GitHub release, with the update package signed by the updater's key and `latest.json`; it stops if a secret is missing or the version is already released. The installed apps can reach those releases only while the repository is public. The installer is Tauri's NSIS template (`windows/installer.nsi`, from tauri-cli 2.12.4: keep it in step with Tauri's) with its own interface instead of the wizard: one window in the app's colours, with a title bar of the same colour on Windows 11, the logo (`windows/installer/logo-*.bmp`, the dark logo drawn for each display scale), what the app does and one amber button: Install, Update or Reinstall, depending on what is already there. A thin amber bar follows, then the app opens by itself; an app already running is closed without asking. It installs in `%LOCALAPPDATA%\Programs\Drezimo Soundboard`, the per-user programs folder: under Program Files every update would ask for administrator rights. It adds the Start menu and desktop shortcuts; its texts (`windows/installer/texts.nsh`) follow the system language, Italian or English. The uninstaller has the same window, with an option to delete the settings too. The installer adds the login item, which starts the app in the tray (`--autostart`); turned off in the app or in Windows' startup apps it stays off, and uninstalling removes it. Only the published builds (made with `DREZIMO_OFFICIAL`) update themselves. While nobody uses the app, at start and then every 6 hours (10 minutes after a failure, such as no network yet at login), the app looks for a new version, downloads it and installs it. Nobody uses it when the board is not being updated, the window is closed or minimised, and the board has not been touched since the app started or for 10 minutes. The installer then shows a small progress bar for a few seconds and opens the app again, in the tray if it had started with the system. The installer is not code-signed, so Windows asks about an unknown publisher at the first install only. The updater's private key lives outside the repository and as a repository secret: lost, it would leave the installed apps without updates.

### Voicemod integration: to verify early

> Status: done — official key obtained and Control API verified on 2026-10-29 (results below).

Before implementing all the logic, check the installed Voicemod version and the actual availability of an official/local API, any access keys and the commands to trigger sounds. Do not take for granted that the API notifies the start and end of each sound.

If no suitable integration is available, consider hotkeys configurable in Voicemod. The STOP behaviour and the "playing" state depend on the features actually exposed.

**Research of 2026-10-29 (to verify on the PC):** there is an official Voicemod Control API ([reference](https://control-api.voicemod.net/api-reference/)):
- Local WebSocket `ws://localhost:59129/v1/`, opened by the Voicemod app.
- Mandatory first message `registerClient` with a `clientKey` **provided by Voicemod** (contact: devservices@voicemod.net). It is unclear whether any key is accepted: to be tested.
- Useful actions: `getAllSoundboard`, `getMemes`, `getBitmap` (icons), `playMeme` (`FileName` + `IsKeyDown`), `stopAllMemeSounds`, `toggleMuteMemeForMe`.
- `IsKeyDown` is used for "hold" playback modes (`PlayLoopOnPress` etc.): the firmware must send **both press and release** of the pads.
- **No playback start/end events are documented**: the "confirmed" green LED is probably not achievable; to be verified by listening to messages after `playMeme`.
- Global STOP available with `stopAllMemeSounds`.
- Test script: `tools/voicemod_probe.py`.

**Test of 2026-10-29 on macOS, Voicemod 3.17.42:** port 59129 is open and the server responds `Pending authentication`. With a made-up `clientKey`, `registerClient` returns **401 Unauthorized** and the connection is closed. An official key is required: in the meantime, the fallback was the board as a USB keyboard + Voicemod shortcuts.

**Test with official key (2026-10-29, macOS, free licence) — confirmed:**
- `registerClient` → `200 Authorized`. Key in `.env` (`VOICEMOD_CLIENT_KEY`), excluded from git.
- `getAllSoundboard`: 36 soundboards; each sound has `id`, `name`, `enabled`, `playbackMode`, `imageURL`, `favorited`. With the free licence only some are enabled (e.g. "Voicemod Starter" with exactly 16 sounds).
- `getMemes`: 820 sounds; the `FileName` to pass to `playMeme` **matches the sound's `id`** in the soundboards.
- `playMeme` only receives an acknowledgement (`actionType: playMeme`) for the down and for the up. **No playback start/end event** from the Control API in the following 10 s; the real state comes from Voicemod's log instead (see below).
- **`IsKeyDown: false` restarts the sound:** Voicemod treats every `playMeme` as a press. The release must be sent **only** for `PlayLoopOnPress` sounds; for the others only the press is sent (otherwise one-shots play twice and Play/Pause stop immediately).
- **Real state from the log:** `voicemod-desktop.log` records `playSound … id=<id> … repMode=k<Mode>` at start, ``Removes a sound=`<id>` from active sounds`` at the end and `Requested to stop all sounds`. `tools/voicemod_log_bridge.py` tails it and publishes the events on `ws://localhost:59200`, learning durations from the timestamps (e.g. Honk 0.85 s, Error 1.62 s). The file reaches disk with a ~0.5–1 s delay: green lights up immediately on press using the learned duration, and the log acts as confirmation. This is an internal Voicemod format: it may change with updates. On Windows it is in `%LOCALAPPDATA%\VoicemodV3\logs\` (checked on 2026-11-02, see below).
- **Sound duration:** not exposed by the API. Voicemod downloads sounds into a cache (`cache/<Soundboard> - NN - <Name>.ogg`) but the files are obfuscated/encrypted: we do not decrypt them. Durations are learned from the log instead (see below); a configurable fallback is used only without the log watcher. The Community's sounds (`tuna/<id>.ogg`, checked on Windows) are plain Ogg, and the app reads their length from the file.
- After playback, Voicemod spontaneously sent `getAllSoundboard`, `getVoices`, `getMemes`: the app must handle unsolicited messages.

### Voicemod soundboard behaviour (verified)

Design rule: **a pad press is exactly a click on the sound in Voicemod**. All playback semantics run inside Voicemod; the device only sends `playMeme` key-down (plus key-up for `PlayLoopOnPress`) and mirrors Voicemod's real state on the LEDs. Verified on 2026-10-29 with `tools/voicemod_experiments.py`, which records commands, Control API messages and log lines on one timeline.

Sound metadata used (from `getAllSoundboard`): `playbackMode`, `loop`, `stopOtherSounds`, `muteOtherSounds`, `muteVoice`, `enabled`, `favorited`.

| Situation | Voicemod behaviour | Log lines |
|---|---|---|
| Press, sound stopped | Starts | `playSound id=…` |
| Several sounds | Play simultaneously (no limit observed) | one start/end pair each |
| `PlayRestart`, press while playing | Restarts from the beginning; with `loop` it never ends by itself | `processRepMode … soundStatus=kPlaying`, single `Removes` at the end |
| `PlayStop`, press while playing | Stops | `processRepMode … kPlaying` + `Removes` |
| `PlayPause`, press while playing | Pauses; **the sound stays active** | `processRepMode … kPlaying`, no `Removes` |
| Any mode, press while paused | Resumes | `processRepMode … soundStatus=kPaused` |
| `PlayLoopOnPress` | Plays while held; key-up stops it | start on key-down; `processRepMode` + `Removes` on key-up |
| `stopOtherSounds` (start from stopped) | Stops every other sound first | `Requested to stop other sounds…` + `Removes` for each |
| `muteOtherSounds` / `muteVoice` | Handled by Voicemod (ducking / mic mute); nothing in the log | — |
| STOP (`stopAllMemeSounds`) | Stops everything, paused sounds included. There is no per-sound stop in the API: a single sound is stopped through its own mode, as in Voicemod | `Requested to stop all sounds` + `Removes` for each |
| `PlayOverlap` | Each press adds an instance (**not verified yet**: no free sound uses it; test with a custom sound) | — |
| A sound of the Community tab (in no soundboard) | Plays as `PlayRestart` and stops the others; its file is downloaded at the first play as plain Ogg. Its id is the online library's, in neither `getAllSoundboard` nor `getMemes` | `playSound id=<library id>, path=…/tuna/<id>.ogg, repMode=kPlayRestart, stopOthers=true` |
| A licence-locked sound clicked in Voicemod | Not played: Voicemod offers Pro | — |

**Checked on Windows (2026-11-02, Voicemod 3.17.45):** the log is in `%LOCALAPPDATA%\VoicemodV3\logs\`, with the same lines as on macOS, except that a first press has no `soundStatus` and `soundsCount`. A soundboard sound clicked in Voicemod's window logs the same id as the API; a Community sound logs its library id, as above.

How the device mirrors it (`simulator/dummy-board.html`, `tools/voicemod_log_bridge.py`):
- The log watcher rebuilds Voicemod's per-sound state machine (playing / paused / stopped, instance count) and learns natural durations only from uninterrupted plays.
- The log reaches the disk ~1 s late, so a press is applied immediately as a **prediction** using the same state machine; the log lines confirm it by timestamp (same PC clock). Lines nobody predicted (sounds started from Voicemod's own window, ends of sounds with unknown duration) correct the state; a prediction not confirmed within 4 s triggers a snapshot resync.
- LEDs: playing = green, paused = breathing green, loop sounds stay green until stopped. The OLED shows the remaining time, `LOOP`, `PAUSED` and `MIC OFF` while a `muteVoice` sound plays.
- The release of a `PlayLoopOnPress` pad is delayed to at least 200 ms after the press. A key-up sent while Voicemod was still loading a sound (4 ms after key-down) left an **orphan** entry in Voicemod's active list (`soundStatus=N/A`) that is never removed; it disappears only by restarting Voicemod. The watcher drops orphans on STOP / stopOthers.
- Also mirrored: Voicemod's soundboard order (one page per 36 sounds), a Favourites page for `favorited` sounds, the active soundboard (`getActiveSoundboardProfile`), "Mute for me", licence-locked sounds (dim red, not sent).

**How to get the key:** from the [control-api.voicemod.net](https://control-api.voicemod.net/) website → "Get your API Key", [Typeform form](https://voicemod.typeform.com/to/Zh5ZHRED); alternatively email devservices@voicemod.net.

**Target environment:** **Windows** with Voicemod, free or paid. Development and first tests on macOS with free Voicemod; Windows builds checked in CI and on a Windows PC. The bridge app runs on the PC (not on the board), with a one-time installer and auto-start.

### Tools and simulator

- `simulator/dummy-board.html`: the 6×6 layout at 1:1 scale (case 130.3 × 196.35 mm, same layout as the PCB), with function keys, encoder and display, connectable to Voicemod. It also plays the board's own screen at power-up and when the "Bridge app running" switch is off ("Replug USB" replays it), pixel for pixel as the firmware draws it: the logo data is generated together with `firmware/drezimo/logo_bitmap.h`. With **Connect board** (Chrome or Edge, Web Serial) the page plays the bridge app for a real board: DTR on while the "Bridge app running" switch is on, `HELLO`, LED changes as colour and effect, display frames when the picture changes, `BRIGHT` from the brightness slider, and the board's keys and knob drive the page. The module LED can copy any LED of the chain, to see the effects without the PCB.
- `tools/voicemod_log_bridge.py`: Voicemod log watcher, publishes events on `ws://localhost:59200`.
- `tools/voicemod_experiments.py`: records commands, Control API messages and log lines on one timeline.
- `tools/voicemod_probe.py`: Control API probe.

## 9. Board ↔ app protocol

USB serial, plain text: one message per line (`\n`, `\r\n` accepted), fields separated by commas, message name first. Names are uppercase; the board also accepts lowercase. A text field, when present, is always the last one and may contain commas. Lines longer than 2100 characters are dropped. Indices follow the simulator: pads 0–35 row by row from the top-left, function keys 0–5 (PREV, NEXT, STOP, MUTE ME, VOICE, HEAR ME), LEDs 0–35 for the pads and 36–41 for the function keys. The firmware maps them to the physical matrix and LED chain.

### Board → app

| Message | Sent when |
|---|---|
| `PAD,<0-35>,<1/0>` | A pad is pressed / released |
| `BTN,<0-5>,<1/0>` | A function key is pressed / released |
| `ENC,<+1/-1>` | The knob turns one step clockwise / anticlockwise |
| `ENCBTN,<1/0>` | The knob is pressed / released |

The board reports raw inputs only. Page changes, peek, row overview and the soundboard menu are decided by the app, as in the simulator. One exception: the input that wakes a blanked display is swallowed by the board (press and release), so waking the display never plays a sound.

### App → board

| Message | Reply | Effect |
|---|---|---|
| `HELLO` | `DREZIMO,<firmware version>` | Handshake: the app finds the board and decides whether to update its firmware |
| `LED,<0-41>,<r>,<g>,<b>[,<effect>]` | – | Sets one LED. Effect: `SOLID` (default), `BREATHE` (paused sound), `BLINK` (Voicemod unreachable) |
| `BRIGHT,<0-100>` | – | Global LED brightness, applied before the power limiter |
| `DISPLAY,<2048 hex digits>` | – | A full 128×64 frame, 1 bit per pixel, in the SH1106 layout: 8 bands of 8 rows, 128 bytes each, least significant bit on top |
| `WAKE` | – | Restores the display brightness (a Voicemod event, such as a sound started from the PC) |
| `SET,<name>,<value>` | `OK` | A display setting, saved in flash: `OLED_BRIGHTNESS` 0–100, `OLED_DIM_AFTER` and `OLED_OFF_AFTER` in minutes without input (0 = never), `OLED_SHIFT` in minutes (0 = off); minutes go up to 1440. Defaults 50, 1, 10, 3 |
| `SETTINGS` | `SETTINGS,<brightness>,<dim>,<off>,<shift>` | Reads the saved settings back |
| `BOOT` | `OK` | Reboots into the USB bootloader for a firmware update |
| `DEVLED,<r>,<g>,<b>` / `DEVLED,MIRROR,<0-41>` | – | Development only: the module's own RGB LED (GP16), which faces the PCB in the finished device. `MIRROR` copies one LED of the chain, to try the effects without the PCB. Off at power-up |

Invalid messages change nothing and get `ERR,unknown message` (also the sign of a firmware older than the app) or `ERR,invalid arguments`. Messages without a reply are fire-and-forget, so a page change (42 `LED` lines) does not flood the link with acknowledgements.

All of the above is implemented in firmware 0.2.0. Tested on the bare module: every message, the errors, settings kept across a reboot, the LED effects through `DEVLED,MIRROR`, and the whole link driven by the simulator over Web Serial. Keys, encoder, LED chain and display wait for the PCB.

### Division of work

- **The app draws the display.** It renders the 128×64 frame, as the simulator already does on a canvas, and sends it whole. Layout, fonts, localised texts and the PC overlay preview all live in one place, and changing the UI never needs a firmware update. A frame is 2 KB and is sent only when the picture changes (about 10 per second while a countdown runs), which is nothing for USB.
- **The board keeps what must be immediate or work without the PC:**
  - the LED effects and the ≈400 mA power limiter, so animations stay smooth whatever the USB timing;
  - the burn-in protection: dimming, blanking and the 1 px pixel shift (the app leaves a 1 px margin). The timers count from the last key, knob or `WAKE`. Only an input that wakes a *blank* display is swallowed; on a dimmed one it works normally;
  - its own screen, shown at power-up and whenever no app is connected, with no text, so the firmware needs no translations. A light wave crosses the 6×6 grid of the logo and leaves the D lit, the wordmark slides in, then three small pads blink in turn while it waits. The real pads play the same wave and keep the D lit in amber, so the keyboard itself shows the logo; they go dark with the display when it blanks. The animation is drawn by code from the 1 KB wordmark bitmap (`logo_bitmap.h`).
- **Connection:** the board considers the app connected while the serial port is open with DTR on (the app must enable DTR: .NET's `SerialPort` leaves it off by default). On connection the board turns the LEDs off and waits for the app's state; the logo stays on the display until the first `DISPLAY` frame. When the port closes, it shows its own screen again.
- **PC asleep:** while USB is suspended, the display and the LEDs are off.

### USB identity and firmware protection

- The board identifies itself as **Drezimo Soundboard** (manufacturer **Drezimo**). `firmware/build.sh` sets these names at build time. USB ID 2E8A:F00A for now; a dedicated product ID can be requested for free from Raspberry Pi, for open-source RP2040 projects.
- On Windows the Device Manager lists the port as "USB Serial Device (COMx)", the name of Windows' own serial driver. The product name shows in the device's details and in the Settings device list, and the app shows its own status.
- The firmware can only be replaced from the USB bootloader. The board gets there through `BOOT`, a 1200-baud open-and-close of the port (the Arduino upload convention), picotool's reset request, or the BOOT button held while plugging in (inside the case). Even then nothing changes until a firmware file is written. A board left in the bootloader by mistake (it shows up as the RPI-RP2 drive) returns to normal when unplugged, with its firmware intact. The bootloader's own name, RPI-RP2, lives in the chip's ROM and cannot be changed; it appears only during an update, which the app drives.

## 10. Work plan

### Phase 1 — Decisions and verification
- [x] Verify the Voicemod Control API and the log-based playback state.
- [x] Obtain the official Voicemod client key.
- [x] Decide the layout: 6×6 pads + 6 function keys + EC11 encoder + display.
- [x] Choose the display: 1.3" OLED 128×64, SH1106, I²C.
- [x] Decide the power strategy: USB only, firmware LED power limiter ≈400 mA.
- [x] Update the simulator to the 6×6 layout at 1:1 scale.
- [x] Choose the parts (RP2040-Zero, switches, keycaps, encoder, SN74AHCT125N, OLED).

### Phase 2 — Measurements before the PCB
- [x] Measure the switch LED-window offset from the centre with a caliper.
- [x] Measure the OLED module outline, mounting holes and pin order.
- [x] Check the EC11 pin layout from the kit.
- [x] Measure the RP2040-Zero (pin grid, USB-C position, 5V = VBUS, boots with a C-to-C cable).
- [x] Check whether the fab can assemble Kailh hot-swap sockets (yes, but out of stock at order time: switches are soldered instead).

### Phase 3 — PCB design and order
- [x] Schematic: 7×6 key matrix with diodes, SK6812 MINI-E chain (0–41) with 100 nF per LED, SN74AHCT125N, 330 Ω data resistor, 47 µF bulk capacitor, 0.5 A polyfuse, encoder, OLED header, RP2040-Zero (ERC 0, `check_design.py` OK).
- [x] Layout: placed on the measured parts and fully routed (DRC clean, schematic parity 0). 2-layer board, LED cut-outs on the measured LED window, soldered MX switches, DIP-14 footprint, RP2040-Zero under the board with its USB-C at the back edge, GND pours on both layers.
- [x] Check the footprints against the real parts (1:1 printout: switches, keycap spacing, OLED, encoder, RP2040-Zero).
- [x] Generate the fab files (Gerbers, drill, BOM and placement for assembly) and check the parts' availability at the fab.
- [x] Order the boards with SMD assembly on the bottom side (Standard PCBA): 2 assembled v0.2 boards. A first order of 5 bare v0.1 boards went out by mistake.

### Phase 4 — Firmware
- [x] Serial protocol (section 9), settings in flash.
- [x] Matrix scan with 5 ms debounce, encoder with interrupts (written; to be tested on the PCB).
- [x] SK6812 driver with effects and the ≈400 mA power limiter (tested through the module LED).
- [x] SH1106 driver for the app's frames, burn-in protection, animated logo (written; to be tested on the OLED). The texts are drawn by the app, so they are localised there.
- [x] Reboot-to-USB-bootloader command for updates from the app (`BOOT`, tested with `firmware/upload.sh`).

### Phase 5 — Bridge app (macOS and Windows)
- [x] Tauri app skeleton: tray icon, board found automatically, handshake and live activity (`app/`).
- [x] Voicemod Control API in the app: pages of 36 sounds, Favourites, function keys, knob pages, LED states (tested against Voicemod 3.17.43 on macOS).
- [x] Log watcher in the app: real playback state and sound lengths (tested against Voicemod 3.17.43 on macOS).
- [x] Display frames drawn by the app, soundboard menu, peek and row overview (window preview; to be checked on the real OLED).
- [x] Voicemod log on Windows: `%LOCALAPPDATA%\VoicemodV3\logs\`, same lines; Community sounds shown as "Community" with their countdown (2026-11-02, Voicemod 3.17.45).
- [x] The 6×6 PC overlay: names and live LED colours, on page changes and while the knob is held (tested on macOS and on a Windows desktop; to be seen over games on a Windows PC).
- [x] Firmware update from the app: automatic for a board with an older firmware (tested on the bare module: 0.1.9 → 0.2.0 in about 5 s, on macOS).
- [x] Installer with its own interface, per user, with auto-start (checked on Windows 11 at 100% scale with an install, a reinstall, a passive update run and a silent uninstall, 2026-11-02; a real self-update from a release is still to be seen).
- [x] Settings window with a sidebar: the board's LEDs and display, the overlay, the language and starting with Windows (2026-11-02; starting with Windows not yet tried in the installed app).
- [ ] Test connection errors and failed commands.
- [ ] macOS distribution, on a Mac: a DMG in the installer's style (background, window, icon positions), a login item, and the updater's macOS package in the Release workflow. Not signed nor notarized: Gatekeeper's first-launch block is lifted with "Open Anyway" or `xattr -cr`.
- [ ] Later: custom per-sound idle colours.

### Phase 6 — Enclosure and final assembly
- [ ] Design the enclosure (130.3 × 196.35 mm) on the real measurements; test-print a plate section with a few switches.
- [ ] Bring up the assembled PCB at low brightness: LEDs, OLED, encoder, a few switches.
- [ ] Solder the switches, the RP2040-Zero, the encoder, the buffer and the OLED module.
- [ ] Print the enclosure on the P1S, mount everything, close it with the 6 screws, add the rubber feet.
- [ ] Run extended tests on Windows with paid Voicemod with the finished unit.

## 11. Decisions

### Last decisions (2026-10-29)

- [x] Layout: 6×6 = 36 pads + a row of 6 function keys (PREV, NEXT, STOP, MUTE ME, VOICE, HEAR ME) + EC11 encoder + display.
- [x] Voicemod integration: official Control API with key + log watcher for the real playback state.
- [x] Sound behaviour: a pad press is exactly a click in Voicemod; playback modes verified (see section 8).
- [x] Encoder: rotate = page, tap = soundboard menu, hold + pad = peek, hold + rotate = row overview.
- [x] Display: 1.3" OLED 128×64, SH1106, I²C, driven by the RP2040.
- [x] LEDs: 42 SK6812 MINI-E reverse-mount on the PCB bottom; opaque keycaps with halo lighting.
- [x] Key matrix: 7 × 6 with SMD diodes (13 GPIOs, 19 of 20 in total).
- [x] Custom 2-layer PCB with fab-assembled SMD parts; switches, RP2040-Zero, encoder, buffer and OLED module hand-soldered.
- [x] Power: USB only, ≈400 mA LED cap in firmware, 0.5 A polyfuse + 47 µF on the LED rail.
- [x] Microcontroller: RP2040-Zero clone (USB-C) instead of the Raspberry Pi Pico, mounted on its pin strips under the PCB (RP2040 and crystal sit on its underside). PCB and pin map updated to the measured part (2026-11-01).
- [x] USB cable exits from the top (back) edge of the case, not from the left side.
- [x] Enclosure: one-piece print on the P1S, 6 M3 screws into plastic, removable bottom.
- [x] Updates: through the bridge app, board firmware included; nobody needs to touch the board.
- [x] Bridge app: Tauri 2 for macOS and Windows, all logic in Rust, settings window in React (2026-11-01; measurements in section 8).
- [x] No polyfuse on the whole 5 V input: the PC's USB port limits its own current, and the 0.5 A polyfuse already guards the LED rail, which draws the most (2026-11-02; v0.2 is ordered without it).
- [x] No code signing, on Windows or macOS (2026-11-02): the first launch is confirmed by hand, as for Fast Memo.
- [x] Releases: the repository goes public for the first release, and the apps update from its GitHub releases (2026-11-02).

### Still open

- [ ] Enclosure material: PLA or PETG.
- [ ] `PlayOverlap` behaviour (test with a custom sound).

## 12. Safety and testing rules

- Double-check polarity and connections before powering up.
- Do not work on the circuit while powered during soldering or modifications.
- Avoid short circuits between +5 V and GND.
- The device is powered by USB only: do not add a second 5 V supply and do not feed external power into the RP2040-Zero's 5V pin, so the PC's USB port is never back-fed.
- Never power the LEDs from the RP2040-Zero's 3V3 pin.
- Keep the firmware LED power limiter active from the very first power-up; run initial tests at reduced brightness.
- Do not increase the bulk capacitance on VBUS beyond ~100 µF (USB inrush current); the design uses 47 µF.
- Verify the actual pinout of the SN74AHCT125N and of the components received: do not rely on physical position alone.
- Check the OLED pin order on the module's silkscreen before soldering: VCC and GND are swapped between modules (the PCB expects VCC-GND-SCL-SDA).
- Check the LED orientation and footprints before ordering the assembled PCB.
- Test the assembled PCB (LEDs, display, encoder, a few switches) before plugging in all 42 switches, and do not close the enclosure before testing the firmware, the software and the Voicemod integration.

---

## Quick summary

**Goal:** 6×6 physical soundboard for Voicemod: 36 pads, 6 function keys, EC11 encoder, OLED display.

**Controller:** RP2040-Zero (RP2040, USB-C) under a custom 2-layer PCB.

**Communication:** USB serial to a bridge app on the PC, which talks to the Voicemod Control API and watches the Voicemod log.

**LEDs:** 42 SK6812 MINI-E reverse-mount (0–35 pads, 36–41 function keys), with SN74AHCT125N buffer, 330 Ω resistor, 100 nF per LED, 47 µF bulk capacitor and a 0.5 A polyfuse.

**LED feedback:** off = empty; blue = available; yellow ~200 ms = pressed; green = playing; breathing green = paused; dim red = not in licence; blinking red = Voicemod unreachable.

**Display:** 1.3" OLED 128×64, SH1106, I²C; localisable texts (English and Italian).

**Power:** single USB cable; firmware caps the LED current at ≈400 mA.

**Enclosure:** 130.3 × 196.35 mm, one-piece print on a Bambu Lab P1S, 6 M3 screws, removable bottom.

**Next steps:** the app's installer and auto-start; the assembled PCB's bring-up; the enclosure.
