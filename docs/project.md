# DIY physical soundboard for Voicemod

> Note: the project is in English. The OLED display texts can be localised: English and Italian so far: the system's language by default, or the one set in the app.

> Project document  
> Status: the first unit is built and works: PCB v0.2 with its LEDs fab-assembled, firmware 0.3.0, the 42 keys, the LEDs, the display and the knob, closed in the printed enclosure (section 7). The bridge app drives Voicemod, the display, the PC overlay and the board's firmware updates, and installs and updates itself on Windows. Next: the updates tried from a public release, on Windows and on macOS.

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
- 2026-11-02: hardware test on the app's Diagnostics page, for a board just built: every key and the knob ticked as they are used, the LEDs colour by colour at low brightness, and test pictures on the display.
- 2026-11-03: the display redrawn: icons for Voicemod's switches and the playback state, the voice loaded, a bar of what is left of a sound, long names scrolling, pages sliding in, a scrollbar in the menu, a map of the pads when peeking, and screens that say what to do when Voicemod is away.
- 2026-11-03: the app's window: a report to copy for support, the update state and what is new after an update, the overlay's screen, a pixel-exact display preview, a locked pad blinking red with PRO ONLY, menus with a search field, a notice when Voicemod's Mute for me silences the sounds for the user, connection failures tested one by one, no browser menu or shortcuts, and screenshots in the README.
- 2026-11-03: checked on Windows: the overlay on two screens, Escape, no browser shortcuts. A text field's menu now holds only cut, copy and paste there; the Soundboard page's rows give way to a notice instead of the page scrolling; the support report says whether the app starts at login, and gives connection errors in English.
- 2026-11-04: a pad stops only its own sound through Voicemod's bridge, on Windows and macOS; what a pad does when pressed again is chosen for each kind of sound; each of the knob's gestures does what the user picks, Voicemod's volumes included; the Drezimo page in tabs; the configuration exported and imported as one file, with a copy to put back.
- 2026-11-05: starting with the system on macOS too, as a switch named after the system; on Windows, the login item finding the app already open no longer opens its window, and turned off it no longer leaves a mark that outlives an uninstall.
- 2026-11-20: the enclosure printed (top shell with the white logo and labels, bottom), without a stand: the bottom's keyholes take one if wanted.
- 2026-11-21: the first unit assembled and working.
- 2026-11-27: macros as sounds on tracks along a time line, cut where wanted and with their own volume, the actions on a track of their own; the older macros run as they were until edited.
- 2026-11-27: the Macro section's guide, How it works, at the top right of the section: six slides, each with a small time line to try, shown by itself the first time.
- 2026-11-27: a pad of the user's own and a macro can breathe or blink at rest in the colour picked, from 14 colours.
- 2026-11-27: the knob's double tap and long press take the time set, for hands that are not so quick.
- 2026-11-27: Drezimo's display on the PC screen moves from Diagnostics to the Overlay page, saved, shown always or with the actions picked, as the soundboard menu, beside the overlay or where set.
- 2026-11-27: a page of the user's own can be kept without empty pads, the next ones moving up when one is cleared; a page started from a soundboard longer than 36 sounds takes the part picked; the soundboard menu goes round past its ends.
- 2026-11-27: a macro's sounds play over each other and once each whatever Voicemod's own options for them (stop the other sounds, mute them, loop), turned off for the run and put back after; stopping a macro stops its sounds; a volume point at 0 % can be grabbed again.
- 2026-11-27: the action wheel, asked by the first user: a knob gesture opens a wheel of up to 8 shortcuts (pages, actions, macros) on the screen around Drezimo's logo, picked by turning; off until a gesture is set to it.
- 2026-11-27: the knob's volume of the sound playing is kept for its next plays, the knob can move both soundboard channels as one fader, a sound pad of the user's pages has its own volume, and the overlay and putting the volumes back are pads too.
- 2026-11-27: from the first user's feedback: the board shows the page open in the Pages editor, and the Soundboard page opens one of the user's pages there; a pad dragged below the page is cleared, the selected pad has a ⋯ menu; a sound's mode in Voicemod is changed from the editor, so any sound can play as overlapping copies; the Drezimo page's Pages tab picks which of Voicemod's pages the board shows (only the user's own soundboards, say); the overlay can stay on screen, a knob gesture hiding and showing it.

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
- A pad press is exactly a click on the sound in Voicemod, unless the user picks what a pad pressed again does (section 8, "Pressing a pad again").
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

There is no per-sound stop in the Control API: a single sound is stopped through its own playback mode, as in Voicemod, or through what the user picked for its pad, with Voicemod's undocumented bridge where found (section 8).

### EC11 encoder

Incremental encoder with continuous rotation and integrated push button (3 GPIOs, 3.3 V logic). The app recognises its gestures (`knob.rs`) and does for each the action picked on the Drezimo page's Knob tab (`knob` in `settings.json`, `controller/knob.rs`); the defaults are the knob as it always was:

| Gesture | Recognised as | Default |
|---|---|---|
| Turn | a step while the knob is up | change page |
| Hold + turn | a step while the knob is down | row overview (below) |
| Tap | down and up with no turn and no pad | soundboard menu (choose the active soundboard) |
| Double tap | a second press within 0.3 s of a tap's release (200–1000 ms, set on the Knob tab), let go before the long press's time with no turn and no pad | nothing |
| Long press | down for at least 0.6 s (400–2000 ms, set on the Knob tab) with no turn and no pad, acted on when let go | nothing |
| Hold + press a pad | | peek (below); not configurable |

- While double tap does nothing, a tap acts at once; otherwise it waits 0.3 s for a second one, and a second press that turns, peeks or is held 0.6 s lets the first tap act first, then counts on its own. While long press does nothing, any hold let go with no turn and no pad is a tap: holding to look at the overlay, then letting go, opens the menu.
- The open soundboard menu takes the turns and the taps, whatever is picked, going round past its ends (since 2026-11-27); holding the knob still shows the overlay.
- **Action wheel** (`wheel.rs`, `controller/wheel.rs`, `overlay/wheel.rs`): off until a gesture is set to it. Its 2 to 8 slices, clockwise from the top, are what a press can do (a page, an action, a macro, the soundboard menu; never the wheel again), kept in `wheel` in `settings.json` and in the backup. Set on the turn while held, the first step opens it on the first slice that way (the last one turning left), each step moves on, the centre (Drezimo's logo) between the last slice and the first, and letting the knob go does the slice turned to: at the centre nothing happens, not even a tap. Set on the plain turn, or on a tap, a double tap or a long press (which open it at the centre), it waits: the turns move, any press does the slice and closes it, and it closes by itself 8 s after its last turn. It closes when the board goes away, and the menu closes it. While open it takes the page overlay's place on the PC screen, drawn natively as the mockup approved on 2026-11-27 (a dark disc, the slices with the window's icons in the colour of a pad that does the same, the one turned to in amber, the logo, the slice's name under it, a hint below), at the overlay's place, size and opacity unless it has its own (anchor, 60–130 %, 40–100 %), and Drezimo's display shows WHEEL, the slice's name, what it is and the places as dots. It needs Voicemod connected. The Knob tab shows its editor once a gesture uses it: the slices on a wheel to click, each one's kind and target, moved back or on, 2 to 8, its look and a button that shows it on the screen for 3 s, as a change of look does.
- Turns: change page; row overview, which after a plain turn stays 4 s after the last step; the soundboard's volume for the others, for the user, or for everyone (both channels a step together, as one fader: a sound started next starts from there); the microphone's volume, up to 200 %; the volume of the latest sound started that still plays, saved in Voicemod as that sound's own volume once the knob has rested on it 1 s (`ModifySoundOptions`, as Voicemod's own slider saves it), so its next plays start from it (putting the volumes back before then drops that save; a save does not stop the sound, tried on 2026-11-27, while a change of mode does); change voice, the favourite voices in the licence, else all of them, in turn, each shown at once and loaded in Voicemod at most every 250 ms, the last always (a voice loaded meanwhile by a pad, a macro or Random voice cancels the one waiting); the action wheel (below); nothing. Presses: soundboard menu, stop all sounds, the microphone, Mute for me, the voice changer, hear myself, background effects, a random voice, a macro, a page, the overlay (hidden and shown again while it stays on screen, otherwise shown for its time; nothing while it is off), put back Voicemod's volumes, open the action wheel, nothing.
- Volumes go through Voicemod's bridge (section 8), in steps of 5 % of Voicemod's own sliders (gain = (percent/100)², linear above 100 %), at most one message for each channel or sound every 100 ms, the last value always sent. The display shows what changes, a bar and the percent, for 2.5 s. Without the bridge, or before Voicemod told the value, it says NOT AVAILABLE; with no sound playing, NO SOUND. Putting back returns each channel the knob changed to Voicemod's own value, read when the bridge connects and followed when Voicemod's own slider moves, and the sounds still playing to their saved volume. Voicemod's sliders do not move with the knob, and Voicemod sets its own values again when it restarts: the Knob tab says so.
- A macro or a page deleted since does nothing; the Knob tab shows its second select empty, with "no longer there".
- Taps, double taps and long presses show in the Diagnostics page's latest key presses, after the knob's own presses and turns.

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
| Voice available | Purple | A voice pad, on the Voices pages |
| Voice loaded | Green | The voice Voicemod uses now, confirmed by Voicemod |
| Action, on | Its own colour | An action pad of the user's pages, while what it switches is on: mic muted red, voice changer and hear myself green, mute for me amber, background effects cyan; the overlay pink while it shows; stop red while sounds play; random voice purple, go to a page and putting the volumes back white always |
| Action, off | Its colour at a quarter | The same pad while what it switches is off |
| Macro, at rest / running | Its colour, solid / breathing | A macro pad: amber unless the macro or the pad has its own colour |
| The user's colour | Picked in the editor | Replaces a pad's colour at rest (blue, purple, an action's own) on the user's pages; playing and loaded stay green |
| Not in licence | Dim red | Sound or voice locked by the licence; not sent |
| Not in licence, pressed | Blinking red, 1.2 s | Nothing plays, as Voicemod would not play it nor say so; the display shows PRO ONLY, the error list names the sound; a page change ends the blink, as it ends the yellow flash |
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
- Contents: page name + counter, last sound, its state (play, pause, loop icons) with a bar of what is left and the seconds, a macro's steps, the voice loaded and Voicemod's switches as icons; sound names for peek and row overview.
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
- The finished PCB was measured on 2026-11-18 and the shell is drawn on those readings (`hardware/enclosure/drezimo_case/measurements.py`): the deck's top face is 8.2 mm over the board, set by the highest corner of the OLED glass (5.83 mm), and the window follows the lit area as read from the edges of the display's own board, by caliper and off a photo of the lit display (the mean of the two, which were half a millimetre apart). The knob's lower edge, pressed, stays 3 mm over the deck, so the encoder's threaded bushing shows under it. The module's bottom pair of holes is still a photo estimate; nothing uses it.
- The enclosure is generated from the PCB's own numbers by `hardware/enclosure/` (build123d, in a `uv` environment). `cd hardware/enclosure && uv run python -m drezimo_case coupon-a` writes the print files (STEP and 3MF, deck face-down, black body and white inlay as two parts) and a preview picture to `hardware/enclosure/print/`. `inlay-gauge` in place of `coupon-a` writes a flat test plate for the 0.2 mm nozzle: white bars and black gaps from 0.2 to 1.0 mm wide, the six function-key labels at three sizes, and the logo three times at the deck's width, as drawn and with strokes and dots drawn 0.1 and 0.2 mm bigger to make up for what printing takes off. `shell` writes the case's top part, and only once every reading it uses is entered as measured; until then `shell --draft` draws it on the estimates as a picture (`shell-draft.png`, not committed) and writes nothing that can be printed. With `--bambu` after the name, and Bambu Studio installed (macOS), the command also writes `<name>-bambu.3mf`: a Bambu Studio project for the P1S with the nozzle the part is meant for (0.2 mm for every part, the nozzle the inlays need: it is not changed between parts), the filaments, the plate, the first layer's colour order and the supports already set, written only if Bambu Studio slices it without a warning and, for the shell, with black supports standing nowhere but under the key wells and with more black than white, which a body and an inlay loaded in each other's filament would not give; the shell's deck is printed full through. The shell lies on the plate with its back wall to the back of the bed, where its seam goes, and has no brim.
- The deck must not give under a finger: between the back wall and the keys it is held by ribs 1.2 mm wide, at most 10 mm apart both ways, that stop 0.5 mm over the board. Every part that stands in that strip keeps its room: the ribs are lowered over the USB module's pin ends, and a wall goes round the chip, the display's module and the encoder.
- Heights come from the caliper values in `hardware/enclosure/drezimo_case/measurements.py`; the command lists the ones that are still estimates, and final parts refuse to build on them. A value is no estimate once it is measured, or `known` from elsewhere with its source written next to it. The shell's 3MF, and so its Bambu Studio project, hold it bigger than modelled by what the printed inlay gauge says the printer shrinks (`P1s`, `P1l`): the smaller of what its two sides say, never smaller than modelled and at most 0.25 % bigger. The STEP and the picture show it as modelled. `uv run pytest` checks the geometry on nominal values; `uv run pytest tests/test_shell.py --as-measured` runs the shell's checks on the values in that file, which is the last check before a shell is printed.
- The bottom (`uv run python -m drezimo_case bottom`) is a 1.8 mm plate that goes inside the shell's rim, flush with its end. Six standoffs take M3 x 10 countersunk screws from below, each through a counterbore to a 45° seat, and clamp the board against the shell's bosses; the seat is drawn for the longest screw of the set (10.4 mm, head included), whose tip stays 0.6 mm short of the end of its hole. The standoffs reach the board 0.2 mm before the plate reaches the shell's seat, so a board a little thicker or thinner than 1.6 mm is still held. Ribs 1.2 mm wide run along every line between the keys, both ways, up to 1 mm under the board, and a post stands 0.2 mm under it at each of the 30 points between four keys: neither the plate nor the board gives under a finger. Everything that hangs under the board is read from the KiCad file (LEDs, diodes, capacitors, the switches' pins and pegs, the other parts' pin tips, the USB module), and nothing of the bottom comes within 0.5 mm of it sideways or 0.3 mm under it. Under the USB module no rib can stand, so the plate is thicker there instead (3.4 mm, 0.9 mm under the connector), from 3 mm behind the connector's mouth, where the plug's grip comes in: nowhere on the plate does a circle of plain plate wider than a key fit. Four keyholes under keys 7, 12, 25 and 30 take the pegs of an optional stand, not designed (neck 4 mm, head 7 x 2 mm, 8 mm of slide towards the case's front), each with a closed pocket behind it, 3 mm high inside. The four 20 mm square rubber feet are glued flat on the plate's face beside the corner screws, each inside a groove 0.6 mm wide and 0.2 mm deep that shows its place: a recess would print as a bridge as long as the foot. It prints with its outside face on the plate, in black, without supports, full plastic all through, with the same 0.2 mm nozzle as the shell (it has no inlays, but the nozzle is not changed between the two parts), in 0.1 mm layers: its heights are given to a tenth of a millimetre, and at 0.2 mm the standoffs would end a tenth short of the board with the posts level with them. A Bambu Studio project whose layers do not end at the part's own height is refused, for any part. The screw and the feet are readings like the others (`M9l`, `M9d`, `M9k`, `M10w`, `M10h`): the bottom waits for them, the shell does not; `bottom --draft` draws it on estimates as a picture only.

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

On macOS the app lives in the menu bar, never in the Dock: `LSUIElement` in `app/src-tauri/Info.plist`, which Tauri merges into the bundle's. Set only at run time, the app went back to the Dock when opened again from the Finder while running.

Board link (`app/src-tauri/src/board.rs`): every second the app lists the serial ports with the board's USB ID (2E8A:F00A) and opens each one with DTR on; a port that answers `HELLO` with `DREZIMO,<version>` is the board. The ID is arduino-pico's generic one, so a port that leaves 3 handshakes unanswered (3 s apart) is probably another device and is then tried only every 30 s, in case it was a board whose answer got lost; a dedicated product ID would make the filter exact. The handshake starts with a newline, which ends any half line a broken session left in the board's buffer. A port that is listed but cannot be opened is held by another program: it is reported as *in use* and retried every 3 s. One thread reads and writes: Windows runs a serial port's reads and writes one at a time, so reads wait at most 25 ms and messages for the board go out between them (replies at once); writes may wait up to 1 s for room in the USB buffers. An unplugged board is noticed from the read errors or, where a driver reports none, within 2 s from asking the port for its state.

Settings window (`app/src-tauri/src/window.rs`): built on demand, hidden with the app's dark background until the page reports its first complete frame (about 170 ms on an M-series Mac, release build), so it never flashes white. A sidebar leads to the pages and says at its foot, in a few words, whether Drezimo is connected and Voicemod running, with Settings under it. Soundboard (the main page, called Status until 2026-11-02: the display's live picture beside the title, at a size where each of its pixels takes a whole number of the screen's pixels (`useCrispSize`, 128×64 points on a Retina Mac), three times as big once clicked, until a click or Escape; the knob beside it, played as the device's through the board's own gestures (`VirtualKnob.tsx`, `turn_knob` and `press_knob`): the mouse wheel turns it a step a notch, counted with the legacy `wheelDelta` so the system's lines to scroll do not matter (a touchpad's moves add up to one, a gesture's first move is always one; a move away from the user turns it right, with macOS's natural scrolling too; a sideways move counts only with Shift), a click presses it, a click held is a long press and held while the wheel turns is a turn held down, with the focus on it the arrows turn it and Enter or Space press it, and Shift held while a pad is pressed or a step turns holds it down, so Shift and a pad peek at the pad (Shift alone presses nothing, so Shift+Tab is no tap; only while the window has the focus, and let go at the next move without Shift, as a key-up in another program never comes); Drezimo's state as a dot beside the title and a grey line below it, what it means and what to do (connected, not connected, in use by another program, its firmware updating or the update failed), with its display on the right; a notice when a sound plays that the user does not hear, since only the others hear it and the cause is easy to miss in Voicemod: its Mute for me on (once: "Fine as it is" keeps it, and either answer is kept in the settings, `muteForMeNoticeSeen`), or the soundboard's volume for the user at zero in Voicemod's mixer, read through its bridge (until the app starts again, as it is no choice to remember); "Hear them again" (`hear_sounds_again`) turns Mute for me off, Voicemod bringing the channel back at the user's own volume, or, with the volume at zero, turns the channel on and up to 100 % through the bridge as Voicemod's mixer does; "Fine as it is" on the volume's notice holds in the app until it starts again (`close_volume_notice`); the current page as the board's 6×6 pads, with their names, Voicemod's pictures and live LED colours, playable from the window as from the board, held clicks included, with the page picked from a menu without showing the overlay (the app's own menu, `SearchSelect.tsx`, the same on Windows and macOS: a search field focused at the top, ignoring case and accents and matching anywhere in the name, the current page checked and in sight, arrows, Home and End, Enter, Escape and Tab (both close it, the focus back on the button; in a dialog, Escape closes the menu only), the list following the keys but not the pointer, and a letter typed on the closed button opens it with that letter; a short list, as what a pad does when pressed again, has no search field, and a letter there moves to the first option starting with it; the soundboard filter of the sound pickers, the voice of a macro step and the page a pad or a macro step opens use the same menu); STOP, MUTE ME, VOICE and HEAR ME as icons beside the page arrows, lit as on the board and explained on hover: all of it works without the board, with Voicemod alone; with nothing to play yet, a small picture of the board beside what to do: a soft wave over its dark pads while Voicemod connects or sends its soundboards, every key blinking red as on the board while Voicemod is closed or refuses the app, empty pads when there is no page at all), Pages (the user's own pages, below), Macros (below), Drezimo (the device's own settings, called Board until 2026-11-02, as the window calls the device Drezimo everywhere, in tabs under the title, by mouse or with the arrow keys, Home and End, the open one kept while the window stays open: Pads, what a pad does when pressed again for each kind of sound (below); Knob, what each of its gestures does (section 3); a ▶ beside each Pads and Knob row opens how it works, a looping animation of the gesture and of what it does with the current settings (a double tap or long press with no action of its own shows the tap's, as the board does, a quick macro run twice included; a macro or page not chosen, deleted or with no steps shows that nothing happens, and why; a pad shows what the app makes of the choice for that kind of sound), with sample names and the row's own select, loaded only when first opened (`app/src/explain/`), its sound bars drawn about 30 times a second, still while the window is hidden or minimised (WebView2 is told of a minimised window, which it would not know) and without movement when the system asks for less; Pages, which of Voicemod's pages the board shows after the user's own (Favourites, Voices, only the soundboards made in Voicemod, each soundboard with its own switch); LEDs and display, the LEDs' brightness, shown live while the slider moves, and the display's settings, read from the board and saved there with `SET` when a control is let go), Overlay (on or off, one of nine places, size and opacity shown live while their sliders move, duration or always and a preview), once switched on, Diagnostics (a report to copy for support, the board's port and firmware, whether Voicemod's log is read and its bridge found, the hardware test and the latest key presses: only what the other pages do not show, so not the display, already beside the Soundboard page's title; and, until 2026-11-27, the display on the PC screen, since then on the Overlay page (below), `overlay/display.rs`: the picture the app sends Drezimo's display, the hardware test's too, each dot 2 or 3 points wide in whole pixels, in a native window like the overlay's, above the others, the overlay included when it comes up in the same place, and letting the mouse through, at one of the overlay's nine places on its screen, so it stays in sight while the pages and tabs change or the window is closed; below it the latest inputs, the newest first: pads as R3 C2, keys by their printed names, the knob, its steps added up while they come the same way within 1.5 s, amber while held, dimmed 4 s after their last change, in grey when from the window, let go when the board goes away; its screen looked for again every 3 s while shown; saved since 2026-11-27 with the settings: never, always, or with the soundboard menu open, a volume the knob changes, the knob held (a peek, the row overview, the action wheel) or a page change, each picked, and 1.5 s after (2.5 s after a page change); at the overlay's place and screen, beside the overlay rather than over it (above it at the bottom, below it at the top, beside it on the sides) and 3 times as big with an overlay of 90 % or more, or at a place and size of its own; with a button that shows it 3 s; in the backup), and at the foot Settings (the language, starting with Windows, the welcome tour, the versions, the backup, the Diagnostics switch; called General until 2026-11-04). The app's own settings are a JSON file in its config folder (`settings.rs`, `%APPDATA%\dev.drezimo.soundboard\settings.json` on Windows); a missing field takes its default, so older files still load. Starting with the system is a switch named after the system, available only in the installed app. On Windows, turning it off removes the login item, so an update does not bring it back; turning it on also clears what Windows' startup apps hold for it, where it can be turned off too, which an update keeps. On macOS it is a launch agent, `~/Library/LaunchAgents/dev.drezimo.soundboard.plist`, which launchd reads at the next login: off until turned on, since no installer asks, and read as off if it names another path, as after moving the app. System Settings can turn it off among the login items, which the app cannot see. There the installed app is an app bundle that is neither on its disk image nor in the random copy macOS runs of an app opened where it was downloaded. A closed window is only hidden for 5 minutes, for an instant reopen, then destroyed to give its memory back. It opens from the tray icon's menu and, on Windows, with a click or a double click on the icon (the menu then opens with a right click, as usual there). The app opens it when started by hand and stays in the tray when started with `--autostart`, as the login item does, unless the welcome tour has not been seen yet. Each window gets a new label, so one being destroyed never clashes with a new one. It behaves as an app, not a web page (`app/src/guard.ts`): a right click opens no browser menu except in text fields (cut, copy, paste), the browser's shortcuts do nothing (reload with F5 or Ctrl/Cmd+R, print, find, view source, save, open, zoom, caret browsing with F7), a file or a link dropped on it is not opened as a page, and the middle button does not start Windows' autoscroll.

Support report (`report.rs`, "Copy report" on the Diagnostics page): plain English text for the owner to paste in a chat when something goes wrong on another PC: the app's version and whether it is a published build, the system and its version, the language, the board (port, firmware, an update in progress or failed), Voicemod (its own version, read from the `appVersion` its messages carry, the licence, whether its log is read and whether a single sound can be stopped through its bridge), how many pages and macros, where the app's updates are, the main settings (the overlay's screen among them) and the 20 latest errors with their time; an error repeated in a row takes one line, with how many times and the last time. When the system refuses the copy, the page shows the text to select and copy by hand. Every failure the app prints also goes to that list (`errors.rs`), a pad outside the licence and a file that cannot be saved (settings, pages, macros) among them; a folder in a message is cut down to the file's name, since a user's folder names the user.

Welcome tour (`app/src/tour/`): eight slides in a dialog over the window, shown in its first frame while `tourSeen` is false in the settings and again from Settings: the board and its parts, the LEDs' colours (a colour picked dims the pads that do not use it), the display and the overlay, a small Pages editor, a macro tried step by step, the Drezimo page (its Knob and Pads tabs, each row tried in its ▶ explanation, whose code loads with the tour rather than with the window; nothing is saved), and the Settings page where the tour stays and the backup is. They show a fixed sample page, so they work without the board or Voicemod, and nothing in them reaches either. The display's screens and the overlay are drawn by the app itself (`tour.rs`) from that page, in the window's language, so they look exactly as on the board and the PC; the rest reuses the window's own components. The slides are laid out at 708 × 318 points and scaled down to fit a smaller window; the arrow keys move between them and Escape closes the tour. Skipping it or closing it with Escape also marks it seen, with a notice of where to find it again; so does closing the window while it shows. The board is drawn with its logo in one colour, white as printed on it.

Hardware test (`board_test.rs`, run by `device.rs`; `app/src/pages/BoardTestCard.tsx` on the Diagnostics page): for a board just built. Started from the page while the board is connected, it takes the board from the soundboard: the controller is told the board is away, so nothing plays and it sends no LEDs or screens, the keys and the knob go to the test instead, and a macro running stops. The page shows the board drawn as in the tour (`app/src/Drezimo.tsx`), each key with a tick once pressed and lit as on the board, a bar of the 42 keys, and ticks for the knob turned both ways and pressed. The LEDs show the keys (white while held, then green) or one colour on all of them (red, green, blue, white) to find a dark or wrong LED; the LED brightness is held at 30 % at most meanwhile. The display shows a frame with text in the middle, every pixel lit, or a checkerboard, all inside the 1-pixel margin the screens keep: the pixel shift moves the picture by a pixel and leaves the edge it uncovers dark, which would look like dead pixels. The window's display preview shows the same picture. The app sends `WAKE` with each new picture and every 30 s, so the display neither dims nor turns off while it is looked at, and a first key press is never swallowed to wake it. After a reconnection the board gets everything again, and what was held counts as released. The test ends with Finish, when the page is left, the window closed or the page reloaded: the LEDs get their brightness back and the controller sends every LED and its screen again, without the overlay a connection shows. "Start over" clears the ticks for another board.

Custom pages (`custom_pages.rs`, the window's Pages editor): the user's own pages of 36 pads, in `pages.json` in the app's config folder, shown on the board before Voicemod's. A pad holds a sound or a voice, by Voicemod's id (its name kept too, for the editor while Voicemod is away), or an action: stop the sounds, mute the microphone (`toggleMuteMic`), the voice changer, a random voice among those in the licence other than the current one, hear myself, mute the sounds for me, the background effects (`toggleBackground`), go to a page, the overlay (as the knob's gesture: lit pink while it shows) or put back the volumes the knob changed. It can also have its own name on the display and the overlay, its own LED colour at rest (14 to pick from) and its movement at rest, steady, breathing or blinking, as the board animates them already (`animation`; a sound playing is still green, a macro running still breathes, or blinks when it breathes at rest; other movements would need a firmware update tried on a board first) and, for a sound, what it does when pressed again (below) and its own volume (`volume`, in percent; none for Voicemod's own): sent with `updateSoundVolume` right after the key-down that starts it, through the bridge only, for that play only and only on a press that starts the sound (not one that pauses, resumes or stops it), so the knob steps from it and the sound keeps its own volume elsewhere, unless the knob rests on another one while it plays. A sound or voice Voicemod no longer has leaves its pad empty on the board and shows as no longer in Voicemod in the editor; a pad the app does not know, from another version, is read as empty rather than losing the page. The editor: the pages as tabs (new, blank or from a soundboard's 36 sounds, the part picked of a longer one as the board splits it, named with it, such as "Memes 2/3", rename by double click or the menu, duplicate, move, delete with a second click, and kept without empty pads, a menu option saved with the page (`compact`): a pad cleared, the ones after it move up one, and a pad moved goes after the others rather than to an empty place), the board going to the page the editor shows, so a page is tried on the board as it is edited (once there the board is free to move on; a new page goes there once the board has it), opened on the board's page when that is one of the user's, or on the one the Soundboard page's edit button names; the 6×6 grid as on the board (click an empty pad to fill it, a full one to select it, drag to move or swap, drag below the page, where a bin shows while dragging, or Delete to clear; the selected pad has a ⋯ menu in its corner with change and clear), a bar for the selected pad (name, a sound's mode in Voicemod, when pressed again, LED colour, page to open, change, clear; the mode is changed in Voicemod for good, on every soundboard with the sound, through its bridge, as Voicemod's own window does, and shows once Voicemod sends the soundboards again, about a second later; without the bridge it is only shown; a pad with its own choice for pressed again shows it as a small amber mark) and a picker (sounds with search and soundboard, those in the licence first; voices, licensed first; actions; macros to come) where several choices fill the clicked pad and the free pads after it. Changes are saved 300 ms after the last one. Voicemod's censor beep (`setBeepSound`) is left out: Voicemod 3.17.45 answers it `commandNotImplemented`.

Macros (`macros.rs`, run by `controller/running.rs`, the window's Macro section, `pages/MacroTimeline.tsx`): what a pad of the user's pages plays, kept in `macros.json`, as sounds on up to 6 tracks along a time line (`timeline`: each clip a sound by Voicemod's id, its track, when it starts, where it is cut and its volume along the way as levels in percent, straight between them and flat after the last; at most 50 clips in the first ten minutes) and actions on a track of their own, each at its time: load a voice, a random one in the licence or the one loaded when the macro started, set one of Voicemod's switches (microphone, voice changer, hear myself, mute for me, background effects) on, off or the other way, stop the sounds or open a page. A sound starts with a key-down, as a pad's; at the same time a cut and an action come before a start, so a stop then a sound plays the sound. The cut is the bridge's stop of that sound, and the volume its `updateSoundVolume`, sent at most every 100 ms while it changes: without the bridge a sound plays to its end at its own volume, as a stop all would stop the others too. A round ends with its last event once its sounds have ended, or 30 s after it for a sound that never ends. Voicemod plays each sound with options of its own, which 405 of its 858 library sounds set to stop the other sounds when they start, 46 to mute them while they play, and its music to loop (read on 2026-11-27): a horse over thunder cut the thunder, and a song played on forever. So, for a run, a macro turns those options off for its sounds through the bridge (`ModifySoundOptions`, all of a sound's options from those last read, however old, as Voicemod's window tells each change: `controller/overrides.rs`), while the app's own idea of what plays follows the options the macro gives (measured that day: Voicemod answers a change in 0.1–0.2 s, the first after a while idle in 1.2–1.4 s, and a change does not stop a sound that plays). Only a clip that would stop or mute another of the macro's sounds already playing waits for Voicemod's answer, 1.5 s at most, the rest of the time line moving on by as much so its distances stay as drawn; a sound that loops and is not cut stops at its length all the same, should its loop still be on. They are put back once the sound is quiet and no macro running uses it, from options read afresh, sent again every 5 s and at each new connection until Voicemod confirms, and kept until then in `sound-overrides.json` in the config folder, so an app closed halfway puts them back at its next start. Voicemod answers these changes without saying which sound, and to every client: the app takes the answers in order, dropping a change unanswered for 3 s. A new bridge connection reads the options at once when a macro has a time line. Stopping a macro (pressed again, the editor's Stop, another macro starting, starting over) stops the sounds it started, one by one through the bridge, else every sound. Nothing makes a sound start from the middle, pause it or play it twice at once: Voicemod has no command for the first two, and the third only for a sound whose own mode stacks copies, so the editor marks a sound that plays over itself in red. A macro from before the time line (`steps`: a sound, the next step at once or once it ends; a wait; the same actions) runs as it was until it is edited: the editor shows it laid out on the time line (a sound the next step does not wait for on the next free track, an unknown length as 3 s) and saves it as a time line, without its steps. A macro is on as many pads as wanted, edited in one place; pressed again it stops or starts over; it runs once, 2 or 3 times or until stopped (a round of instant steps at most twice a second); at the end it can put back the voice and the switches it changed. One runs at a time: another one stops it. While it runs its pads breathe in its colour and the display shows its name and `STEP n/m`, counting the sounds and actions started. The editor has the macros as tabs; the time line with the seconds along the top, an amber band where two sounds or more play at once and an amber line where a sound or an action added next goes (clicked on the seconds; after a sound is added, at its end); sounds drawn as long as they are, from the lengths Voicemod reads from their files (`GetAllResources`' `duration`, asked when the section opens, at most once a minute, else the lengths the log taught, else 3 s with a dashed border); a sound dragged along its track or onto another, its right edge dragged to cut it, its volume shaped as Voicemod's own, steady, fading in, out or both, at a level, or drawn point by point (double click to add, right click to remove); an action dragged along its track; Delete removing what is selected; a right click on a track adding a sound there (on that track when it is free there) or an action at that time, and on a sound or an action duplicating it right after itself or deleting it, the menu opening towards the side with room; Shift+click selecting several, which move, duplicate (right after the block they make) and go together, a click without moving on one of them selecting it alone; the arrow keys moving what is selected by a tenth of a second, a second with Shift, Up and Down changing its track, Escape letting go; a move or a cut landing on the starts and ends of the others, the actions, the cursor and the start within 8 points, else on the tenths; undo and redo of the last 50 changes of the macro shown (Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z anywhere on the page but in a text field, and two buttons), a change that changes nothing not counted; a sound with Voicemod's own option to stop the other sounds, starting while others play, marked in amber, with what it cuts and a link that turns that option off in Voicemod (`ModifySoundOptions`, `stopOtherSounds`); a zoom of four steps; and, while it runs, a green playhead from when the round started (`roundStarted` in the status) and the sounds playing in green. Try runs it on Voicemod as a pad would. How it works, at the top right of the section (`pages/MacroGuide.tsx`), explains it all in six slides in the tour's dialog, the text on the left and on the right a small time line to try, with the box of what is selected, its own undo and a sample of four sounds (one that stops the others), so it works without Voicemod and nothing in it is saved or reaches Voicemod; it shows by itself the first time the section opens after the welcome tour (`macroGuideSeen` in the settings, kept on each PC, not in the backup), and a short sound on the time line gives its room to the name, its length going to the tooltip. Switching the microphone makes Voicemod play its own notice sound; the log follower leaves those out (ids starting `00000000-0000-4000-`), or they would show as a Community sound.

Backup (`backup.rs`, the Settings page's Backup card): the configuration as one plain JSON file, to keep aside or take to another computer, saved and opened through the system's own dialogs (`tauri-plugin-dialog`, JSON only), which open in the folder used last, else in Documents: left to itself, Windows opens the app's working folder, its program folder or a system one. The file holds `"drezimo": "configuration"`, a `format` (1), the app's version, when it was exported, and four parts: `settings` (the language, the LEDs' brightness, pressing again by kind, the knob, the overlay; not what belongs to one computer: the tour seen, the notices answered, Diagnostics shown, the last page), `pages` and `macros` (as `pages.json` and `macros.json`), and `display`, the board's own settings, only when Drezimo was connected. Never Voicemod's pictures, the error list, the Voicemod key or paths. An import reads at most 5 MB, refuses a file that is not JSON, not a Drezimo configuration or of a newer format, each with its own sentence, and checks each part with the app's own rules for its files: unknown fields ignored, missing ones at their default, numbers brought into the window's ranges, pages and macros through the same filters as at start, pads past the board's and steps past the fiftieth cut before they are read and counted; a part of the wrong type is left out alone. The window then shows when and by which version it was exported, a switch per part and what was left out, counted. The checked file stays in memory until the user confirms or cancels, so a file changed meanwhile cannot slip in. On confirm, the references the import breaks (knob gestures, pads, steps pointing to one of the user's pages or macros that will not be there, when they came with the file or pointed to one there before) become nothing and the result says how many; one broken already stays as it was; the current settings, pages, macros and display settings are copied to `before-import/` in the config folder; the parts are applied as an edit in the window is, at once everywhere, each file written aside then renamed. Display settings go to Drezimo if it is connected, else the result says they were skipped. "Put back as it was" applies the copy the same way, until the next import, keeping what belongs to this computer. Errors go to the error list with the file's name only.

Pad pictures (`pictures.rs`): Voicemod's own, 216 px PNG or JPEG files asked for with `getBitmap` the first time a pad shows them, and kept in the app's cache folder (`%LOCALAPPDATA%\dev.drezimo.soundboard\pictures` on Windows) under the checksum that ends their `imageURL`, so a changed picture is asked for again. The window loads them from its own `pad-image` scheme. Voicemod carries out its actions one at a time (a status request sent after 30 pictures was answered after all of them, 3.3 s later), so the app asks for one picture at a time, the page shown last first: a pad pressed meanwhile waits for one picture at most. A page seen for the first time fills in within about three seconds, its kind's icon showing until then; Voicemod never answers for an id it does not know, which then shows the icon only. A voice's picture has a transparent background and gets a soft light behind it; a locked pad shows its picture in grey.

Voicemod link (`app/src-tauri/src/voicemod.rs`): the Control API WebSocket, registered with the client key, which is built into the app from `VOICEMOD_CLIENT_KEY` (environment or the repository's `.env`, never in git; a build without it shows the missing key). After registering, the app asks for the voices, the soundboards, the licence and the three switches, and follows Voicemod's own messages from then on (soundboards edited, soundboard picked in Voicemod, voice loaded, switches changed; a new licence makes it ask for the voices and the soundboards again). When Voicemod is not running the app retries every 3 s, every 60 s after a rejected key; requests made meanwhile are dropped, and the LEDs blink red.

Failures and what the user sees (`errors.rs` keeps the latest for the report; tried on macOS on 2026-11-03 with the release build, the bare RP2040-Zero and its OLED, unless marked):

| Failure | What the user sees | Tried |
|---|---|---|
| Voicemod closed while the app runs | Within 3 s every pad and key blinks red, the display says OPEN IT ON THE COMPUTER, the window shows "Open Voicemod" beside the board blinking red; the report lists the lost connection | Quit from its menu |
| Voicemod opened again (or for the first time) | The app connects by itself (it tries every 3 s) and shows the same page as before | Reopened: its API answered 2 s after launch, the app followed within seconds, same page |
| Voicemod refuses the app's key | The display says UPDATE THE APP, the window "Voicemod refused the connection", the sidebar's dot red; tried again every 60 s | A debug build with an invalid key: refused at start and again 60 s later |
| Voicemod's log cannot be read | The sounds' lengths are estimated; Diagnostics says the log is not read | Not tried: it means changing Voicemod's own files |
| A request Voicemod drops (unknown or locked id) | Voicemod says nothing ("Failing requests" above); a pad outside the licence blinks red and the display says PRO ONLY | Locked pads pressed from the window |
| Drezimo unplugged | The window says it is not connected; what plays in Voicemod goes on, and pads held from the window stay held until released there (unit tests) | Unplugged by hand |
| Drezimo plugged in again | It connects within seconds and gets every LED and its screen again | Replugged by hand |
| Drezimo's port held by another program | The window says it is in use by another program; it connects within 3 s once the port is free | The port held exclusively by a script for 25 s |
| Firmware update interrupted | Over the serial port: the firmware stays as it was until the whole image is checked, and the OTA bootloader starts the copy again after a power cut. Through the bootloader (boards older than 0.3.0): see firmware updates below, the window says why, and asks to replug when the board waits in its bootloader | Not tried: it needs the cable pulled within the second of the copy |

Device logic (`controller.rs`, run by `device.rs` on its own thread): the simulator's behaviour, written as plain logic with unit tests. Pages come from `soundboard.rs` (the user's own first, then Favourites, the Voices pages and every soundboard in Voicemod's order, 36 pads per page), Voicemod's as the Drezimo page's Pages tab shows them (`shownPages` in the settings: Favourites and Voices each on or off, only the soundboards the user made in Voicemod, `isCustom` in `getAllSoundboard`, and soundboards left out by id; a page that opens a page left out, from a pad, a macro or the knob, does nothing, and the selects no longer offer it). The Voices pages hold only the voices in the user's licence, the favourite ones first: a free licence would otherwise fill seven pages with locked voices. A voice pad loads its voice (`loadVoice`); the voice loaded, wherever it was picked (`voiceChangedEvent`), is lit green once Voicemod confirms it (a first load can take a second), and a voice picked on the computer shows its name on the display as a sound started there does. The licence (free or pro, `getUserLicense`) shows in the window. The current page is kept when the soundboards change (a soundboard that shrank keeps its last page), when Voicemod comes back after a restart and when the app starts again (`last_page.rs`, `page.json` in the config folder). With no page to go back to, the board starts on the first page that is not a Voices page: a first press there plays a sound, rather than changing the voice. A pad sends `playMeme` key-down, or what the choice for a pad pressed again sends (below); the key-up goes only to `PlayLoopOnPress` sounds, at least 200 ms after the press, also when the board is unplugged while the key is held. It never goes to a sound stopped meanwhile (STOP, a sound that stops the others), because Voicemod would take it as a press and start the loop again; a pad pressed again before its key-up went out simply counts as still held. The releases of While held follow the same rules, except stop all, which goes out at once (below). `playback.rs` replays Voicemod's per-sound rules to predict what is playing, and the log confirms or corrects it (below). Only the LEDs that change are sent, and all of them when the board connects.

Log watcher (`voicemod_log.rs`): follows `voicemod-desktop.log` (macOS: `~/Library/Application Support/VoicemodV3/logs/`; Windows: `%LOCALAPPDATA%\VoicemodV3\logs\`) every 100 ms, reopening it for each read so Voicemod can rotate it, and going on from the start of a rotated file (a file missing for less than 2 s is taken as a rotation). At start it reads the last 4 MB: the uninterrupted plays there teach the sounds' lengths, and the lines since the last `------ RUN ------` (Voicemod's start) tell what is playing now. A sound whose file is plain Ogg (Vorbis or Opus), as the Community's are, gets its length from the file the line names, read once, until a whole play measures it: so its countdown shows from its first play. The soundboards' files are obfuscated and only the log teaches their lengths; so does the simulator, fed by the Python bridge, for every sound. The lines are replayed through Voicemod's per-sound rules, as `tools/voicemod_log_bridge.py` did, and matched by timestamp with the predictions of the presses (local times, converted with the system time zone): a line within 600 ms of a prediction confirms it, a line nobody predicted (a sound played from Voicemod's window, the real end of a sound) updates the state unless a press changed that sound later, and a prediction left unconfirmed for 4 s makes the app take the state from the log again. A `------ RUN ------` line (Voicemod restarted) clears everything. After 3 predictions in a row left unconfirmed, the log is taken as unreadable (a Voicemod update may change its format) until the app restarts. Without the log, the app guesses 3 s per sound and the window says so. Learned lengths are not saved yet: the log's own history covers them.

Display (`display/`, `controller/screen.rs`): the app draws the 128×64 frames with the simulator's font, Silkscreen at 8 and 16 px (SIL Open Font License, `display/OFL.txt`), its glyphs captured from the simulator's own canvas; a star was added for Favourites. The screens were redrawn on 2026-11-03 from an approved mockup, so the simulator now draws the older ones. Everything stays inside the 1-pixel margin, accents included. The icons (`display/icons.rs`) are the window's own, redrawn pixel by pixel, most of them 9 pixels high. The home screen shows the page and its number, big the name of the last sound played (for 8 s, then of the sound still playing, if any; PRO ONLY, SOLO PRO in Italian, after a pad outside the licence), and below it the latest sound's state: a play, pause or loop icon, a bar of what is left, when its length is known (for a loop, of the current round, the pauses left out), and the seconds rounded up to a tenth, with the language's decimal separator; a paused time blinks. A countdown wakes the app at each tenth and as often as its bar can move, no more than 25 times a second. A running macro shows its steps as squares (done, the current one blinking, still to come), or a bar when there are too many. At the bottom, under a dotted line, the voice loaded (a flat wave and "voice off" while the voice changer is off) and every switch away from usual as an icon, from the right: microphone muted (also while a sound that mutes it plays), sounds muted for this computer, hearing oneself. The soundboard menu has a scrollbar and a mark on the page the board shows; a peek shows the pad on a small map of the 36, and a name too long for the big letters in the small ones; a row shows its columns as numbered squares and its empty pads as a dash. While Voicemod is away the display says what to do ("open it on your PC", "update the app"), with dots lighting up while it connects. What moves (`display/animation.rs`) does at 25 frames a second, and a frame goes out only when its pixels change: a name too long for the display scrolls by about four letters a second, after a 1.2 s pause at each end; what blinks starts lit; a page turned with the page keys or the knob slides in over 200 ms (from the right going forward, also past the last page; from wherever the display is, if another slide is under way), the bottom strip staying still. A page that changes number only because the list changed, or one picked from the menu, does not slide. The texts follow the language set in the window, the system's by default (Italian or English), as do the window, the overlay and the tray menu. Frames go to the board and to the window, which previews the display. A sound started from the computer sends `WAKE`, so a dimmed display brightens; one outside the soundboards, played from Voicemod's Community, has no name the app can know and shows as "Community". The knob works as in the simulator: a tap opens the soundboard menu (a turn moves, another tap picks, it closes after 6 s); held with a pad press it peeks at the sound (name, what it does when pressed again: its Voicemod mode or the choice, STOPS ALL where that stops every sound, length; no sound played; the pad blinks); held and turned it shows a row of six (the row lights white, the others dim).

PC overlay (`overlay/`, `controller/overlay.rs`): the current page as a 6×6 grid, each pad with its sound's name and its live LED colour, under the page's name and number. Blinking and breathing LEDs animate at the board's pace, though not in step with it: the firmware times its effects from its own start. It appears for 4 s when the page changes (knob, PREV/NEXT, menu, a soundboard picked in Voicemod) or the board connects, and stays while the knob is held or the menu is open, so holding the knob also shows it; it fades in and out, and hides while the board or Voicemod is missing. The Rust core draws it itself, in Inter (SIL Open Font License, `overlay/OFL.txt`; subset to Latin with its kerning kept, names on two lines at most), into a native window rather than a webview, so it never sleeps and appears at once: on Windows a layered window with per-pixel transparency that lets the mouse through, never takes the focus and stays out of the taskbar and Alt+Tab; on macOS a borderless window on every Space. It sits on the screen the mouse is on when it appears, clear of the taskbar or the Dock, at that screen's own scale; the window can set the main screen or another one instead (`OverlayScreen`, which keeps the system's name, made unique with a number when two identical monitors share it; a screen not connected gives way to the main one). The window names the other screens by where they are beside the main one, "Screen on the left · 1920×1080", as the system's names say little (`\\.\DISPLAY2` on Windows, whose numbers often differ from those in Windows' settings, `Monitor #<model>` on macOS), and lists a screen chosen but not connected as such. It goes in the bottom-right corner unless another of nine places is set in the window, which also sets its size (70–100 %: smaller, the names are too small on a screen at 100 %) and opacity, shown live while their sliders move, and its duration, or keeps it on screen (`always`: a knob gesture set to the overlay hides it and shows it again, a page change while hidden shows it for its duration, and a new choice on the Overlay page shows it again), or turns it off; dragging it is not done yet. macOS gives the mouse's position in the main screen's pixels but looks for a screen in points, so the position is turned into points first. It shows over windowed and borderless games and over most "fullscreen" games on Windows 10/11; it does not inject into game processes, like Discord's overlay does, because anti-cheat systems could flag that. Tested on macOS, and on a Windows desktop (a GitHub Actions runner, reading the screen back): the transparency, the half-opacity fade, the click-through window style and the focus staying where it was; still to be seen over games on a Windows PC.

Firmware updates (`firmware.rs`): the app carries the board's firmware, built into it from `app/src-tauri/firmware/drezimo-<version>.uf2` and the `.bin` beside it (written by `firmware/build.sh --bundle`; the build stops if that version differs from the firmware's source). When a board with firmware 0.3.0 or later connects with an older version, the app sends it the `.bin` over the serial port with `UPDATE` (section 9, `board.rs`): no drive shows on the PC, the window and the tray say so meanwhile, and the board is back with the new version a few seconds later. An older board gets it through the chip's bootloader, once, since 0.3.0 takes `UPDATE`: the app sends `BOOT`, waits for the RP2040's bootloader drive (RPI-RP2) that appears next, writes the UF2 file there and waits for the board to come back with the new version, in about 5 s; the window and the tray say so meanwhile. Only a drive that was not there before `BOOT` is written, so another RP2040 board waiting in its bootloader is left alone. A board with the same or a newer version is never touched. A board that comes back with its old firmware, for whatever reason, is not tried again until the app restarts, so it never loops between its bootloader and the app; it keeps working, and the window shows why the update failed. If the bootloader's drive never appears (removable drives blocked on that PC, say), the board waits in its bootloader and the window asks to unplug it and plug it back in, which starts its old firmware again. The bootloader is in the chip's ROM, and the copy takes about a second: a board unplugged before it starts keeps its old firmware, and is updated at the next start of the app; one unplugged in the middle of it can be left without a working firmware, and then needs the BOOT button inside the case and a new copy. On Windows the drive is found among the removable drives, without the "insert a disk" box for an empty card reader; on macOS in `/Volumes` (copying onto the drive by hand can hang there, the app's single write did not in testing). On macOS the drive is not written: the app waits for it to mount, unmounts it, and writes the firmware through the bootloader's own USB interface (PICOBOOT, `picoboot.rs`, through `nusb`), because macOS warns that a disk was not ejected properly whenever the drive goes away as the board restarts. It erases the first flash sector first and writes it last, after reading back and comparing all the others: a board cut off in between has no valid boot code and starts again in its bootloader, ready to be written at the next start of the app, without the BOOT button. Should that interface not open, the file is copied to the drive as on Windows. Tried on 2026-11-21 on the finished unit: about 3 s, no warning.

Installer, start at login and updates (`updates.rs`, `windows/`, `.github/workflows/release.yml`): the Release workflow, run by hand from `main`, builds the Windows installer (NSIS, installed for the current user, no administrator rights) and the macOS disk image (Apple Silicon) with the Voicemod key, then publishes both at once as a GitHub release, with each platform's update package signed by the updater's key and `latest.json`, written by the workflow itself; it stops if a secret is missing or the version is already released. An app finds a new release only once it carries its own platform's update. The installed apps can reach those releases only while the repository is public. The installer is Tauri's NSIS template (`windows/installer.nsi`, from tauri-cli 2.12.4: keep it in step with Tauri's) with its own interface instead of the wizard: one window in the app's colours, with a title bar of the same colour on Windows 11, the logo (`windows/installer/logo-*.bmp`, the dark logo drawn for each display scale), what the app does, four options and one amber button: Install, Update or Reinstall, depending on what is already there. The options are a desktop shortcut, a Start menu shortcut, starting with Windows and opening the app when done: all ticked for a new installation; over an installed version they show what is there, and unticking one removes it. A thin amber bar follows, then the app opens by itself if asked; an app already running is closed without asking. It installs in `%LOCALAPPDATA%\Programs\Drezimo Soundboard`, the per-user programs folder: under Program Files every update would ask for administrator rights. Its texts (`windows/installer/texts.nsh`) follow the system language, Italian or English. The uninstaller has the same window, with an option to delete the settings too. With its option ticked the installer adds the login item, which starts the app in the tray (`--autostart`), on again if Windows' startup apps had it off; turned off in the app or there it stays off through updates, and uninstalling removes it. Updates keep the shortcuts and the login item as they are. Only the published builds (made with `DREZIMO_OFFICIAL`) update themselves. While nobody uses the app, at start and then every 6 hours (10 minutes after a failure, such as no network yet at login), the app looks for a new version, downloads it and installs it. Nobody uses it when the board is not being updated, the window is closed or minimised, and the board has not been touched since the app started or for 10 minutes. On Windows the installer then shows a small progress bar for a few seconds and opens the app again, in the tray if it had started with the system; on macOS the app is replaced where it is and starts again. Settings shows where the updates are (`UpdateState`, the `update-state` event), except in a development build, where the row is left out: waiting for a moment nobody uses the app, looking, up to date with the time of the last check (and its day, when not today), downloading, ready to install, failed with its time, or the updater unable to start (until the next start). Beside it, "Check for updates" looks at once, without waiting for a moment nobody uses the app (`check_for_updates`), and once a version is downloaded "Restart and update" installs it at once (`install_update`); neither runs while the board's firmware is being written. The workflow's `notes` input goes in the release's text above a `---` line, the installer's sentence below it (without notes there is no line, and nothing for the apps to show); `latest.json` carries the whole text and the app keeps only what is above the line (`news.rs`). Written to `news.json` right before the install, it shows once on the Soundboard page of the version installed, "Updated to version …" with the notes, until closed; news of another version is dropped at start. The installer is not code-signed, so Windows asks about an unknown publisher at the first install only. The updater's private key lives outside the repository and as a repository secret: lost, it would leave the installed apps without updates. The macOS disk image (`macos/dmg.py`, built by `dmgbuild`, which writes Finder's layout without driving Finder, so it holds on CI where Tauri's own disk image loses it) opens on a window in the installer's colours: the logo, the app and the Applications folder with an amber arrow of the logo's square dots between them, and one line saying to drag it there (`macos/dmg-background.png` and its `@2x`, drawn from the logo by a local script). Finder writes the icons' names in black over a background picture, in dark mode too, so each name sits on a light plate. The app is signed ad hoc (`signingIdentity: "-"`), as Apple Silicon requires of an app from the Internet, but not by Apple: the first time, macOS refuses to open it until System Settings → Privacy & Security → Open Anyway.

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
- **Failing requests (2026-11-03, Voicemod 3.17.43): Voicemod never says a request failed.** `playMeme` and `loadVoice` get the same empty acknowledgement for a valid id, an unknown one and one locked by the licence. An unknown id makes Voicemod look it up in its online library and log `Unable to download manifest`; a locked sound or voice is ignored without a trace. Only an unknown action gets `commandNotFound`, and an unsupported one `commandNotImplemented` (both with `commandName` empty). So the app cannot learn of a refusal: it knows the locked sounds and voices from their `enabled` field and does not send them, and an id Voicemod no longer has leaves its pad empty.

### Voicemod soundboard behaviour (verified)

Design rule: **a pad press is exactly a click on the sound in Voicemod**, unless the user picks another behaviour for a pad pressed again (below). All playback semantics run inside Voicemod; the device only sends `playMeme` key-down (plus key-up for `PlayLoopOnPress`) and mirrors Voicemod's real state on the LEDs. Verified on 2026-10-29 with `tools/voicemod_experiments.py`, which records commands, Control API messages and log lines on one timeline.

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
| `PlayOverlap` | Each press adds an instance (**not verified yet**: Voicemod 3.17.43 on macOS, free licence, offers no way to set a sound's mode, and a sound uploaded with "Carica suono" comes as `PlayStop`; to be tried where the mode can be set) | — |
| A sound uploaded by the user (2026-11-03, macOS) | `PlayStop`, stopping the other sounds first: played from the app, a second press stops it, the LED and the countdown following it | `playSound … repMode` + `Requested to stop other sounds…`; on the second press `processRepMode` + `Removes` |
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

### Pressing a pad again

What a pad does when it is pressed again while its sound plays is the user's choice (`press_again.rs`): on the Drezimo page for each kind of sound, that is each of Voicemod's modes (sounds that restart, songs that pause, songs that stop, while held, overlapping copies), and in the Pages editor for a sound pad of the user's own pages, whose own choice comes first (its menu shows the kind's choice as As its kind). The default, As in Voicemod, follows the sound's own mode, as before; a sound whose mode the app does not know always does as in Voicemod. The Control API cannot change a sound's mode, nor stop, pause or resume a single sound (the reference checked again on 2026-11-03), so each choice is made of key-downs, key-ups and `stopAllMemeSounds`, and, where Voicemod's bridge is found (below), the bridge's stop of a single sound. What a choice can do depends on the sound's own mode. Voicemod's window keeps each sound's mode, and so do the macros' steps.

What goes to Voicemod when the pad is pressed while its sound plays or is paused (a sound not playing always gets one key-down). Without the bridge:

| Choice | `PlayRestart` | `PlayStop` | `PlayPause` | `PlayLoopOnPress` | `PlayOverlap` |
|---|---|---|---|---|---|
| As in Voicemod | key-down | key-down | key-down | key-down, key-up on release | key-down |
| Start over | key-down | two key-downs: it stops, then starts | stop all, key-down | as in Voicemod | stop all, key-down |
| Stack copies | as Start over | as Start over | as Start over | as in Voicemod | key-down |
| Stop | stop all | key-down | stop all | key-down; no key-up, so it plays until pressed again | stop all |
| Pause and resume | as in Voicemod | as in Voicemod | key-down | as in Voicemod | as in Voicemod |
| While held | as Start over; stop all on release | as Start over; key-down on release | as Start over; stop all on release | as in Voicemod | as Start over; stop all on release |

With the bridge, "stop one" replaces both the stop all and the stopping key-down, so no choice stops the other sounds:

| Choice | `PlayRestart` | `PlayStop` | `PlayPause` | `PlayLoopOnPress` | `PlayOverlap` |
|---|---|---|---|---|---|
| Start over, Stack copies | key-down | stop one, key-down | stop one, key-down | as in Voicemod | key-down (Stack copies), stop one, key-down (Start over) |
| Stop | stop one | key-down | stop one | key-down; no key-up | stop one |
| While held | as Start over; stop one on release | as Start over; stop one on release | as Start over; stop one on release | as in Voicemod | as Start over; stop one on release |

- **Stop all** stops the other sounds too, paused ones included; without the bridge it is the only way to stop a sound whose mode starts it again. The window says so, following whether the bridge is there: on the Drezimo page each kind lists only the choices that do something different for it, As in Voicemod first with what Voicemod does (a choice that comes to Voicemod's own behaviour, or to another choice's, is left out), each marked where it also stops the other sounds, a kind's choice other than As in Voicemod has an amber border, under each kind how many of the user's sounds (those in the licence) are of it and the first names, greyed when none (nothing before Voicemod first sends its soundboards), and under them how many pads have their own choice, with a link to the Pages editor; the editor's menu tells it for the pad's sound, option by option; the peek shows the choice, and STOPS ALL where it stops everything.
- A release goes out at least 200 ms after the press, as the key-up does, and only while the sound still plays; one that stops every sound goes out at once, or it would also stop a sound started meanwhile (a stop all sent 5 ms after a play still stops that sound: tried).
- A pad's stop all also ends a running macro, as STOP does; otherwise the macro would start its next sound at once. A pad that stops only its own sound leaves the macro going, as when that sound ends by itself.
- Every choice but As in Voicemod and Pause and resume relies on what the app predicts plays. Where a sound's length is not known yet (its first play, or Voicemod's log unreadable, when 3 s is guessed), a press right after it ended may, without the bridge, stop every sound instead of playing it (Stop), start and stop it at once (Start over on a `PlayStop` sound), and a release may start it again (While held); the next press then works as expected. With the bridge, stopping a sound that already ended does nothing: no other sound stops, Start over then simply starts it, a release leaves it alone, and Stop on a `PlayStop` sound keeps its own key-down, which starts it as wanted. A second press within the first 0.7 s of a sound's first play in a session, while Voicemod still loads it, cannot stop it: Start over then gives Voicemod a press it takes by the sound's own mode (a pause, say), which the log then shows.
- The choice is saved by name: in `settings.json` one per mode (`pressAgainByMode`: `playRestart`, `playPause`, `playStop`, `playLoopOnPress`, `playOverlap`), and on a pad in `pages.json` (`pressAgain`); one this app does not know, from a newer version, reads as As in Voicemod for that kind, and as none on a pad; one a kind's menu does not offer, from a file edited by hand, reads as what it comes to for that kind; a `pressAgainByMode` that is not an object reads as As in Voicemod for every kind, the other settings kept. A settings file from before 2026-11-04 had one choice for every pad (`pressAgain`): it is read as the same behaviour for each mode, As in Voicemod where it came to Voicemod's own, so no pad changes; the file is written in the new form at the next change.
- Every command goes through `playback.rs` with Voicemod's own rules, so the LEDs, the display and the countdown show what Voicemod actually does, and the log confirms it as for any press. The window's Soundboard page plays its pads the same way.

**Voicemod's bridge** (`voicemod_bridge.rs`): besides the Control API, Voicemod runs a WebSocket between its own window and its engine, on 127.0.0.1 and a port that changes at each start of Voicemod, without any key: 3.17.45 on Windows and 3.17.43 on macOS, with the same messages. It is undocumented: found on 2026-11-04 in the strings of `Voicemod.exe` (its engine's `StopSoundHandler`) and by trying messages. The app looks for it every 5 s while it is not connected: it lists the TCP ports in the listening state on 127.0.0.1 (Windows: `GetExtendedTcpTable`; macOS: `lsof`, run only while a Voicemod process is there, found with `proc_listallpids` and `proc_pidpath`), keeps those of Voicemod's process (`Voicemod.exe`; on macOS `Voicemod.app/Contents/MacOS/Voicemod`, the launcher in Applications and the app it updates in Application Support) except its fixed ports (59129 for the Control API, 39998 and 39999: a WebSocket's handshake on 39998 made Voicemod quit on macOS), and takes the one whose first message within 1 s is a `bridgeData`. On Windows the bridge greets every client with the state of Voicemod's window; on macOS it says nothing until asked, so there it is asked at once to stop the sound `drezimo-probe`, which it answers with an error in about 10 ms (Voicemod's log then shows that stop and its error, and nothing else changes). A port that does not answer so is tried again for 2 minutes, in case Voicemod is still starting, then left alone until its process ends. Besides that probe it sends the stop of one sound: `{"bridgeData":{"data":{"id":"drezimo-<n>","object":{"id":"<sound id>"},"type":"stopSound"},"type":"appMessage"}}`, answered in 10–150 ms by a `stopSound` message carrying `frontendMessageId`, with an `error` (`FailedToStopSound`) when the sound is not playing. Requests to the Control API wait for that answer (at most 1 s; while the bridge is missing, the stop becomes a stop all at once), because the two sockets are separate: a key-down sent 60 ms after the stop was handled before it, and paused the sound instead of starting it again. A stop that fails because Voicemod has not loaded the sound yet (a first play in a session takes up to 0.7 s: pressed, and not reported playing since) is sent again when the bridge reports it `playing` (`soundStateChanged`), within 3 s and unless the sound is pressed again meanwhile. Voicemod's log shows a stop as `Requested stop sound with id=…`, read as the sound's end (its play cut short teaches no length; the end line that follows is then left out, or it would undo a press made since), and a press right after it as ``Play sound=`…` without reproduction mode logic``, a start from the beginning with no end line before it, which still stops the other sounds of a sound that stops them (tried). Whether it is found shows on the Diagnostics page and in the report. Tried on macOS on 2026-11-04: found within 0.2 s, and with two songs playing a stop ended only its own. It may change or go away with any Voicemod update: then the app finds no bridge and the pads stop every sound, as without it. The bridge also reports each sound's state (`created`, `ready`, `playing`, `paused`, `finished`). On each connection the app reads the gains of three mixer channels (`GetVolumeGain`: `streamingSoundboardPreMix`, the soundboard for the others; `monitorSoundboardPreMix`, for the user; `input`, the microphone) and, only while a knob gesture is on the volume of the sound playing, every sound's saved volume (`GetAllResources`, about 2 MB; Voicemod asks its own servers for it, and they answer 429 to requests too close together, so the app asks at most once a minute: when the bridge is found, when that gesture is chosen, and when a sound's volume is not known; a failed answer leaves the known volumes as they were), and follows the `SetVolumeGain` messages it broadcasts, with each channel's `enabled` (a read gives 0 for a channel muted, so that 0 is no volume while the channel is off: the knob says it is not available rather than step from it, and putting the volumes back leaves it; Mute for me is `enabled` on `monitorSoundboardPreMix`, which the Control API's toggle sends too, and its status answers `true` while the user hears the sounds, the contrary of its name: measured with Voicemod 3.17.45 on 2026-11-04, the app reads it so), its own told apart by their `frontendMessageId` (answers to another client's `GetVolumeGain` are left aside: they hold the knob's changes too); it changes a sound's own options for good with `ModifySoundOptions` (`reproductionMode`, `volume`, `loop`, `stopOtherSounds`, `muteOtherSounds`, `muteVoice`, `fadeInDuration`, `fadeOutDuration` and the sound's `id`), which Voicemod takes only all together (a partial one is answered `UnableModifySoundOptions`): the app keeps the options `GetAllResources` gave for 10 minutes, follows those Voicemod's window sends, reads them first when it has none, then asks the Control API for the soundboards again, as Voicemod does not tell it (tried on Windows on 2026-11-27, Voicemod 3.17.49, free licence: a sound set to `playOverlap` played two copies at once, a change of mode while a sound plays stops it, and the mode selector of Voicemod's own window has no licence check); it sets a gain with `SetVolumeGain` and the channel's `enabled`, its mute in Voicemod's mixer, as Voicemod's window sends it (sent without it, the window takes the channel as on, and its Mute for me with it, while the engine may not), and a playing sound's volume, for that play only, with the bus message `updateSoundVolume`, for the knob (section 3). Measured on Windows on 2026-11-27 through the levels the bridge broadcasts (`EnableVumeterLevels`, then `VumeterLevelChanged`): each soundboard channel at 0 is silent on its own output (the other channel still plays it), at 0.25 it is 12 dB down, and at 2 it plays as at 1, so Voicemod holds the sounds at 100 % whatever the gain (only the microphone goes to 200 %); a playing sound's volume at 0 silences it, at 0.5 it is 6 dB down, also when sent just before the key-down. Tried on macOS on 2026-11-04: found in 0.15 s, the three gains in 0.3 s and the volumes of 820 sounds in 1.1 s.

Tried with the bridge on 2026-11-04 (scratch script, bridge events and log read back): one stop each, with other sounds playing, on a `PlayRestart` effect, a playing `PlayPause` song, a paused one (it ends, and the next press starts it from the start), a `PlayStop` song that stops the others, and BLEEP held without key-up while a song played: only the target stopped every time. A stop on a sound not playing answers an error and changes nothing. Stop, then key-down once the answer came: a clean start from the start for a playing, a paused and a `PlayStop` song. `PlayOverlap` sounds could not be tried: the free licence has none.

Tried on Windows on 2026-11-03, Voicemod 3.17.45, with a scratch script and the log read back: two key-downs at once on a playing `PlayStop` sound log its stop, then a new `playSound` 50 ms later; `stopAllMemeSounds` then a key-down on a playing `PlayPause` sound starts it from the start; a `PlayLoopOnPress` sound with no key-up plays on, and a second key-down stops it. The 16 sounds in the free licence on that PC (some are in several soundboards): 7 `PlayRestart`, 5 `PlayPause` (music, looped), 3 `PlayStop` (music, looped, stopping the others), 1 `PlayLoopOnPress` (BLEEP); `PlayOverlap` sounds only among the Pro ones. Tried again through the installed app the same night, from the window's pads: Stop on an effect stopped it and the music under it; Start over on a `PlayStop` sound stopped it and started it again 40 ms later; Pause and resume on a `PlayPause` sound; Stop on BLEEP started it with one press and stopped it with the next; While held stopped a `PlayStop` sound with a second click on release, and an effect with stop all.

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
| `BOOT` | `OK` | Reboots into the USB bootloader, for a firmware update of a board older than 0.3.0 |
| `UPDATE,<size>,<MD5>` | `READY`, then `NEXT` for each 4096 bytes, then `UPDATED` | A new firmware (the build's `.bin`) follows as raw bytes, 4096 at a time, each piece sent once the last is answered. The board keeps it in the flash's file system and, with the whole image and the right MD5, restarts; the core's OTA bootloader then copies it into place and starts it. Any error (`ERR,update …`, or 3 s without data) leaves the firmware as it was. Since 0.3.0 |
| `DEVLED,<r>,<g>,<b>` / `DEVLED,MIRROR,<0-41>` | – | Development only: the module's own RGB LED (GP16), which faces the PCB in the finished device. `MIRROR` copies one LED of the chain, to try the effects without the PCB. Off at power-up |

Invalid messages change nothing and get `ERR,unknown message` (also the sign of a firmware older than the app) or `ERR,invalid arguments`. Messages without a reply are fire-and-forget, so a page change (42 `LED` lines) does not flood the link with acknowledgements.

All of the above is implemented in firmware 0.2.1. Tested on the bare module: every message, the errors, settings kept across a reboot, the LED effects through `DEVLED,MIRROR`, and the whole link driven by the simulator over Web Serial. The real OLED was wired to the bare module on 2026-11-03 (VCC to 3V3, SCL to GP29, SDA to GP28): the power-up screen and the app's screens show as in the window's preview, aligned. Keys, encoder and LED chain wait for the PCB. Since 0.3.0 the flash holds 512 KB of file system for `UPDATE` (`flash=2097152_524288` in `sketch.yaml`); the settings stay in the last sector whatever its size, so they survived the move. `UPDATE` tried on the finished unit on 2026-11-21: 0.3.0 → 0.3.1 from the app, settings kept, no drive on the PC.

### Division of work

- **The app draws the display.** It renders the 128×64 frame, as the simulator already does on a canvas, and sends it whole. Layout, fonts, localised texts and the PC overlay preview all live in one place, and changing the UI never needs a firmware update. A frame is 2 KB and is sent only when the picture changes: up to 25 a second while something moves (a countdown, a long name, a page sliding in), about 50 KB/s, which is nothing for USB; the I²C transfer to the panel takes about 25 ms for a whole frame, and the firmware sends only the bands that changed.
- **The board keeps what must be immediate or work without the PC:**
  - the LED effects and the ≈400 mA power limiter, so animations stay smooth whatever the USB timing;
  - the burn-in protection: dimming, blanking and the 1 px pixel shift (the app leaves a 1 px margin). The timers count from the last key, knob or `WAKE`. Only an input that wakes a *blank* display is swallowed; on a dimmed one it works normally;
  - its own screen, shown at power-up and whenever no app is connected, with no text, so the firmware needs no translations. A light wave crosses the 6×6 grid of the logo and leaves the D lit, the wordmark slides in, then three small pads blink in turn while it waits. The real pads play the same wave and keep the D lit in amber, so the keyboard itself shows the logo; they go dark with the display when it blanks. The animation is drawn by code from the 1 KB wordmark bitmap (`logo_bitmap.h`). At power-up it starts once the display is ready, about 0.3 s later: U8g2 waits through the SH1106's reset timing even without a reset pin.
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
- [x] Display frames drawn by the app, soundboard menu, peek and row overview (window preview; checked on the real OLED wired to the bare module, 2026-11-03).
- [x] Voicemod log on Windows: `%LOCALAPPDATA%\VoicemodV3\logs\`, same lines; Community sounds shown as "Community" with their countdown (2026-11-02, Voicemod 3.17.45).
- [x] The 6×6 PC overlay: names and live LED colours, on page changes and while the knob is held (tested on macOS and on a Windows desktop; to be seen over games on a Windows PC).
- [x] Firmware update from the app: automatic for a board with an older firmware (tested on the bare module on macOS: 0.1.9 → 0.2.0 in about 5 s, and 0.2.0 → 0.2.1 on 2026-11-03).
- [x] Installer with its own interface, per user, with auto-start (checked on Windows 11 at 100% scale with an install, a reinstall, a passive update run and a silent uninstall, 2026-11-02; a real self-update from a release is still to be seen).
- [x] Settings window with a sidebar: the board's LEDs and display, the overlay, the language and starting with Windows (2026-11-02; in the installed app, on 2026-11-05: the switch turned off and on with the registry read after each click, and Windows starting the app at login, in the tray, 73 s after the boot).
- [x] Voices pages and the licence (2026-11-02).
- [x] Sound and voice pictures on the window's pads, not on the overlay nor on the display (2026-11-02).
- [x] Custom pages with an editor: pads holding sounds, voices and actions, with their own names and LED colours (2026-11-02).
- [x] Macros, as a pad of the custom pages: sounds, waits, voices, Voicemod's switches, stop and pages, with a Try button (2026-11-02).
- [x] Welcome tour at the first start, again from Settings, with the display and overlay drawn by the app (2026-11-02).
- [x] Export and import of the configuration from Settings, checked as the app's own files, with a copy to put back (2026-11-04).
- [x] Test connection errors and failed commands (2026-11-03, macOS, bare module: the table in section 8). Still for the PCB: real keys held while unplugging, the red blink on the real LEDs. On the Windows PC (2026-11-03): Voicemod closed and opened again, and the port held by another program, as on macOS; the board unplugged there is still to try.
- [x] On the Windows desktop with two screens (2026-11-03, a 2560×1440 screen above a 1920×1080 one, both at 100 %): the overlay with each screen choice and in each of the nine places on the second screen, clear of its taskbar. Still to see: a second screen at another scale, the chosen screen unplugged, and the overlay over games.
- [x] macOS distribution: a disk image in the installer's style, the app and its update package built and published by the Release workflow with Windows', and firmware updates without the warning about a disk not ejected (2026-11-21). Not signed nor notarized: Gatekeeper's first-launch block is lifted with "Open Anyway" or `xattr -cr`.
- [ ] On macOS: the switch for starting with macOS tried at a real login (its code compiled for macOS on 2026-11-05, never run there).
- [ ] Later: custom per-sound idle colours.

### Phase 6 — Enclosure and final assembly
- [x] Design the enclosure (130.3 × 196.35 mm) on the real measurements; test-print a plate section with a few switches.
- [x] Bring up the assembled PCB at low brightness: LEDs, OLED, encoder, a few switches (the app's hardware test, on its Diagnostics page).
- [x] Solder the switches, the RP2040-Zero, the encoder, the buffer and the OLED module.
- [x] Print the enclosure on the P1S, mount everything, close it with the 6 screws, add the rubber feet.
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
- [ ] `PlayOverlap` behaviour: Voicemod 3.17.43 on macOS cannot set a sound's mode (uploads come as `PlayStop`, checked 2026-11-03); to be tried where it can.

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

**LED feedback:** off = empty; blue = sound available; purple = voice available; yellow ~200 ms = pressed; green = playing, or voice loaded; breathing green = paused; dim red = not in licence; blinking red = Voicemod unreachable.

**Display:** 1.3" OLED 128×64, SH1106, I²C; localisable texts (English and Italian).

**Power:** single USB cable; firmware caps the LED current at ≈400 mA.

**Enclosure:** 130.3 × 196.35 mm, one-piece print on a Bambu Lab P1S, 6 M3 screws, removable bottom.

**Next steps:** the assembled PCB's bring-up; the enclosure; the overlay on the Windows PC over games and on a screen at another scale; the macOS installer.
