"""Timed experiments against Voicemod to learn its exact soundboard behaviour.

Each scenario sends Control API commands on a schedule and records, on one
timeline, the commands sent, every Control API message received, and every
SoundboardController line Voicemod writes to its log. Use it to verify how
playback modes, loop, stopOtherSounds/muteOtherSounds and STOP really behave.

Usage:
    pip install websockets
    python voicemod_experiments.py              # run every scenario
    python voicemod_experiments.py honk_restart # run selected scenarios
"""

import asyncio
import json
import re
import sys
import time
import uuid
from pathlib import Path

import websockets

sys.path.insert(0, str(Path(__file__).resolve().parent))
from voicemod_log_bridge import find_log  # noqa: E402
from voicemod_probe import load_client_key  # noqa: E402

URL = "ws://localhost:59129/v1/"
LOG_FLUSH_WAIT = 2.0  # Voicemod writes its log to disk with ~1 s delay

# Scenario steps: (seconds from scenario start, action, sound name or None)
# Actions: down = playMeme key-down, up = playMeme key-up, stop = stopAllMemeSounds
SCENARIOS = {
    "honk_single": [(0, "down", "Honk")],
    "honk_restart": [(0, "down", "Honk"), (0.4, "down", "Honk")],
    "simultaneous": [(0, "down", "Fart"), (0.3, "down", "Honk"), (0.6, "down", "Sad Trombone")],
    "stop_others": [(0, "down", "Fart"), (0.3, "down", "Sad Trombone"), (0.8, "down", "Hey DJ!"), (2.0, "stop", None)],
    "pause_resume": [(0, "down", "Night Call"), (2.0, "down", "Night Call"), (3.5, "down", "Night Call"), (5.5, "stop", None)],
    "pause_then_other": [(0, "down", "Night Call"), (1.5, "down", "Night Call"), (2.5, "down", "Honk"), (4.0, "down", "Night Call"), (5.5, "stop", None)],
    "mute_others": [(0, "down", "Sad Trombone"), (0.5, "down", "Trauma medic"), (2.5, "stop", None)],
    "stop_toggle": [(0, "down", "Living Lo-fi"), (2.0, "down", "Living Lo-fi"), (3.0, "down", "Living Lo-fi"), (4.5, "stop", None)],
    "loop_on_press": [(0, "down", "BLEEP"), (1.0, "up", "BLEEP")],
}

RE_LOG = re.compile(r"\[SoundboardController\] (.*)")


class Timeline:
    def __init__(self):
        self.t0 = time.time()
        self.rows = []

    def add(self, source, text, at=None):
        self.rows.append(((at or time.time()) - self.t0, source, text))

    def dump(self):
        for t, source, text in sorted(self.rows):
            print(f"  {t:7.3f}s  {source:<4} {text}")


def condense_log(line, names):
    m = RE_LOG.search(line)
    if not m:
        return None
    text = re.sub(r", path=[^,]*", "", m.group(1))
    for sound_id, name in names.items():
        text = text.replace(sound_id, name)
    return text


def log_time(line):
    try:
        return time.mktime(time.strptime(line[1:20], "%Y-%m-%d %H:%M:%S")) + float(line[20:24])
    except ValueError:
        return None


async def run_scenario(ws, steps, ids, names, log_path, inbox):
    timeline = Timeline()
    with open(log_path, encoding="utf-8", errors="replace") as log:
        log.seek(0, 2)
        inbox.clear()
        start = time.time()
        for at, action, name in steps:
            await asyncio.sleep(max(0, start + at - time.time()))
            if action == "stop":
                payload, act = {}, "stopAllMemeSounds"
            else:
                payload, act = {"FileName": ids[name], "IsKeyDown": action == "down"}, "playMeme"
            await ws.send(json.dumps({"id": str(uuid.uuid4()), "action": act, "payload": payload}))
            timeline.add("SEND", f"{act} {name or ''} {'' if action == 'stop' else action}".strip())
        await asyncio.sleep(steps[-1][0] + LOG_FLUSH_WAIT - (time.time() - start))
        for at, msg in inbox:
            kind = msg.get("actionType") or msg.get("action") or msg.get("type")
            if kind not in ("getAllSoundboard", "getMemes", "getVoices"):
                timeline.add("API", json.dumps(msg)[:140], at)
        for line in log.readlines():
            if text := condense_log(line, names):
                timeline.add("LOG", text, log_time(line))
    timeline.dump()


async def main(selected):
    async with websockets.connect(URL, max_size=None) as ws:
        await ws.send(json.dumps({"id": str(uuid.uuid4()), "action": "registerClient",
                                  "payload": {"clientKey": load_client_key()}}))
        await ws.send(json.dumps({"id": str(uuid.uuid4()), "action": "getAllSoundboard", "payload": {}}))
        boards = None
        while boards is None:
            msg = json.loads(await ws.recv())
            if msg.get("actionType") == "getAllSoundboard":
                boards = msg["actionObject"]["soundboards"]
        sounds = {s["name"]: s for b in boards for s in b["sounds"]}
        ids = {name: s["id"] for name, s in sounds.items()}
        names = {s["id"]: name for name, s in sounds.items()}

        inbox = []

        async def reader():
            async for raw in ws:
                inbox.append((time.time(), json.loads(raw)))

        reader_task = asyncio.create_task(reader())
        log_path = find_log(None)
        for key in selected or SCENARIOS:
            used = {n for _, _, n in SCENARIOS[key] if n}
            flags = ", ".join(f"{n}: {sounds[n]['playbackMode']} loop={sounds[n]['loop']} "
                              f"stopOthers={sounds[n]['stopOtherSounds']} muteOthers={sounds[n]['muteOtherSounds']}"
                              for n in sorted(used))
            print(f"\n=== {key}  ({flags})")
            await run_scenario(ws, SCENARIOS[key], ids, names, log_path, inbox)
            await ws.send(json.dumps({"id": str(uuid.uuid4()), "action": "stopAllMemeSounds", "payload": {}}))
            await asyncio.sleep(1.0)
        reader_task.cancel()


if __name__ == "__main__":
    asyncio.run(main(sys.argv[1:]))
