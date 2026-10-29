"""Voicemod sound playback state, read from Voicemod's own log file.

The Control API does not report when sounds start or stop, but the app log does.
This script follows the log, rebuilds Voicemod's per-sound state machine
(playing / paused / stopped, instance count) and publishes it on a local
WebSocket (ws://localhost:59200) for the dummy board and, later, the bridge app.

Messages sent to clients (all carry `t`, the log timestamp in epoch seconds;
the log reaches the disk ~1 s late, so clients use `t` to discard stale events):
    {"type": "snapshot", "asOf", "now", "durations", "sounds": [{id, mode, state, count, t}]}
    {"type": "play", "id", "name", "mode", "repeat", "status", "state", "count", "duration"}
    {"type": "end", "id", "state", "count", "played", "duration"}
    {"type": "stopAll"}
Clients can send {"type": "snapshot"} to resync. Natural sound durations are
learned and saved to voicemod_durations.json.

The log format is internal to Voicemod and may change with updates.

Usage:
    pip install websockets
    python voicemod_log_bridge.py [--log PATH] [--port 59200]
"""

import argparse
import asyncio
import json
import os
import re
import sys
import time
from datetime import datetime
from pathlib import Path

import websockets

DURATIONS_FILE = Path(__file__).resolve().parent / "voicemod_durations.json"

# Known locations; the Windows ones still need to be verified.
LOG_CANDIDATES = [
    Path.home() / "Library/Application Support/VoicemodV3/logs/voicemod-desktop.log",
    Path(os.environ.get("APPDATA", "")) / "VoicemodV3/logs/voicemod-desktop.log",
    Path(os.environ.get("LOCALAPPDATA", "")) / "VoicemodV3/logs/voicemod-desktop.log",
]

RE_PLAY = re.compile(r"playSound (\(processRepMode\) )?id=([0-9a-f-]+), path=(.*?), repMode=k(\w+)(?:.*soundStatus=([\w/]+))?")
RE_END = re.compile(r"Removes a sound=`([0-9a-f-]+)` from active sounds")
RE_STOP_OTHERS = re.compile(r"Requested to stop other sounds before playing a sound=([0-9a-f-]+)")
RE_STOP_ALL = re.compile(r"Requested to stop all sounds")


def line_time(line):
    """Timestamp written by Voicemod (epoch seconds), None for lines without one."""
    try:
        return datetime.strptime(line[1:24], "%Y-%m-%d %H:%M:%S.%f").timestamp()
    except ValueError:
        return None


class SoundTracker:
    """Rebuilds Voicemod's per-sound state machine from its log lines.

    Verified with tools/voicemod_experiments.py:
      - fresh `playSound id=` starts a sound;
      - a repeat press logs `processRepMode` with the state *before* the press:
        PlayPause + kPlaying -> paused (it stays in the active list),
        any mode + kPaused -> resumed, PlayRestart + kPlaying -> restarted,
        PlayStop / PlayLoopOnPress + kPlaying -> a `Removes` line follows;
      - `Removes` ends a sound (natural end, stop, stopOthers, STOP all).
    PlayOverlap instance counting is inferred, not yet verified.
    """

    def __init__(self, durations):
        self.durations = durations
        self.active = {}  # id -> {"mode", "started", "state", "count", "interrupted"}
        self.as_of = 0.0  # timestamp of the last log line processed

    def snapshot(self):
        return [{"id": k, "mode": v["mode"], "state": v["state"], "count": v["count"], "t": v["started"]}
                for k, v in self.active.items()]

    def feed(self, line):
        """Returns the event to publish (or None) and updates learned durations."""
        t = line_time(line)
        if t is None:
            return None
        self.as_of = t
        if m := RE_PLAY.search(line):
            repeat, sound_id, path, mode, status = m.groups()
            name = Path(path).stem.split(" - ", 2)[-1]
            info = self.active.get(sound_id)
            if not repeat or info is None:
                info = self.active[sound_id] = {"mode": mode, "started": t, "state": "playing",
                                                "count": 1, "interrupted": False}
            elif status == "kPaused":
                info.update(state="playing", interrupted=True)
            elif mode == "PlayPause":
                info.update(state="paused", interrupted=True)
            elif mode == "PlayRestart":
                info.update(started=t, state="playing")
            elif mode == "PlayOverlap":
                info["count"] += 1
            else:  # PlayStop / PlayLoopOnPress: the Removes line that follows ends it
                info["interrupted"] = True
            return {"type": "play", "t": t, "id": sound_id, "name": name, "mode": mode,
                    "repeat": bool(repeat), "status": status, "state": info["state"], "count": info["count"],
                    "duration": self.durations.get(sound_id)}
        # STOP and stopOthers silence everything else. Real sounds get a `Removes` line right
        # after; orphans (entries Voicemod never removes, e.g. after a key-up sent while the
        # sound was still loading) do not, so they are dropped here.
        if m := RE_STOP_OTHERS.search(line):
            for sound_id in [k for k in self.active if k != m.group(1)]:
                del self.active[sound_id]
            return None
        if RE_STOP_ALL.search(line):
            self.active.clear()
            return {"type": "stopAll", "t": t}
        if m := RE_END.search(line):
            sound_id = m.group(1)
            info = self.active.get(sound_id)
            if not info:
                return {"type": "end", "t": t, "id": sound_id, "state": "stopped", "count": 0, "played": None}
            info["count"] -= 1
            if info["count"] > 0:
                return {"type": "end", "t": t, "id": sound_id, "state": info["state"], "count": info["count"], "played": None}
            del self.active[sound_id]
            played = round(t - info["started"], 2)
            # Natural duration only when nothing interrupted the sound.
            if not info["interrupted"] and played > 0.1:
                self.durations[sound_id] = played
            return {"type": "end", "t": t, "id": sound_id, "state": "stopped", "count": 0, "played": played,
                    "duration": self.durations.get(sound_id)}
        return None


clients = set()
durations = {}
tracker = SoundTracker(durations)


def load_durations():
    try:
        durations.update(json.loads(DURATIONS_FILE.read_text(encoding="utf-8")))
    except (OSError, ValueError):
        pass


def save_durations():
    DURATIONS_FILE.write_text(json.dumps(durations, indent=2, ensure_ascii=False), encoding="utf-8")


def find_log(explicit):
    if explicit:
        return Path(explicit)
    for path in LOG_CANDIDATES:
        if path.is_file():
            return path
    sys.exit("Voicemod log not found: pass --log with the path to voicemod-desktop.log")


def learn_from_history(log_path):
    """Learns durations and the current state from lines already in the log."""
    with open(log_path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            tracker.feed(line)
    save_durations()


async def broadcast(event):
    message = json.dumps(event)
    for ws in list(clients):
        try:
            await ws.send(message)
        except websockets.ConnectionClosed:
            clients.discard(ws)


async def follow(log_path):
    """Follows the file like `tail -f`, handling truncation and re-creation when Voicemod restarts."""
    handle, inode, position = None, None, 0
    while True:
        try:
            stat = log_path.stat()
            if handle is None or stat.st_ino != inode or stat.st_size < position:
                if handle:
                    handle.close()
                    position = 0  # new file: read it from the start
                    tracker.active.clear()
                else:
                    position = stat.st_size  # first run: history was already read
                handle = open(log_path, encoding="utf-8", errors="replace")
                handle.seek(position)
                inode = stat.st_ino
            for line in handle.readlines():
                known = len(durations)
                if event := tracker.feed(line):
                    await broadcast(event)
                if event and event["type"] == "end" or len(durations) != known:
                    save_durations()
            position = handle.tell()
        except FileNotFoundError:
            pass
        await asyncio.sleep(0.05)


def snapshot_message():
    return json.dumps({"type": "snapshot", "asOf": tracker.as_of, "now": time.time(),
                       "durations": durations, "sounds": tracker.snapshot()})


async def on_client(ws):
    clients.add(ws)
    await ws.send(snapshot_message())
    try:
        async for raw in ws:  # clients may ask for a fresh snapshot to resync
            if json.loads(raw).get("type") == "snapshot":
                await ws.send(snapshot_message())
    except (websockets.ConnectionClosed, ValueError):
        pass
    finally:
        clients.discard(ws)


async def main(args):
    load_durations()
    log_path = find_log(args.log)
    learn_from_history(log_path)
    print(f"Log: {log_path}")
    print(f"WebSocket: ws://localhost:{args.port}  ·  known durations: {len(durations)}")
    async with websockets.serve(on_client, "localhost", args.port):
        await follow(log_path)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--log", help="path to voicemod-desktop.log")
    parser.add_argument("--port", type=int, default=59200)
    try:
        asyncio.run(main(parser.parse_args()))
    except KeyboardInterrupt:
        pass
