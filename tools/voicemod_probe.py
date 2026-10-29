"""Probe for the Voicemod Control API (ws://localhost:59129/v1/).

Checks whether the installed Voicemod version accepts connections and
whether the clientKey is accepted, and saves the available soundboards/memes as JSON.

The key is read from --key, from the VOICEMOD_CLIENT_KEY variable
or from the .env file in the project root.

Usage:
    pip install websockets
    python voicemod_probe.py
    python voicemod_probe.py --play 80    # test playMeme on FileName "80"
"""

import argparse
import asyncio
import json
import os
import uuid
from pathlib import Path

import websockets

ENV_FILE = Path(__file__).resolve().parent.parent / ".env"


def load_client_key():
    if os.environ.get("VOICEMOD_CLIENT_KEY"):
        return os.environ["VOICEMOD_CLIENT_KEY"]
    if ENV_FILE.exists():
        for line in ENV_FILE.read_text(encoding="utf-8").splitlines():
            name, sep, value = line.partition("=")
            if sep and name.strip() == "VOICEMOD_CLIENT_KEY":
                return value.strip().strip("\"'")
    return None


def request(action, payload=None):
    return json.dumps({"id": str(uuid.uuid4()), "action": action, "payload": payload or {}})


async def drain(ws, seconds, log):
    """Collects all incoming messages for `seconds` seconds."""
    messages = []
    try:
        while True:
            raw = await asyncio.wait_for(ws.recv(), timeout=seconds)
            msg = json.loads(raw)
            messages.append(msg)
            kind = msg.get("actionType") or msg.get("action") or msg.get("type")
            log.write(raw + "\n")
            print(f"<- {kind}")
    except asyncio.TimeoutError:
        pass
    return messages


async def main(args):
    url = f"ws://{args.host}:{args.port}/v1/"
    out_dir = Path(args.out)
    out_dir.mkdir(exist_ok=True)
    print(f"Connecting to {url} ...")

    with open(out_dir / "messages.jsonl", "w", encoding="utf-8") as log:
        async with websockets.connect(url, max_size=None) as ws:
            await probe(ws, args, out_dir, log)

    print(f"Done. Full log in {out_dir / 'messages.jsonl'}")


async def probe(ws, args, out_dir, log):
    print("Connected. Sending registerClient.")
    await ws.send(request("registerClient", {"clientKey": args.key}))
    registration = await drain(ws, 3, log)
    print(json.dumps(registration, indent=2)[:2000])

    for action in ("getUserLicense", "getAllSoundboard", "getMemes"):
        print(f"-> {action}")
        await ws.send(request(action))
        for msg in await drain(ws, 3, log):
            if msg.get("actionType") == action:
                path = out_dir / f"{action}.json"
                path.write_text(json.dumps(msg, indent=2, ensure_ascii=False), encoding="utf-8")
                print(f"   saved {path}")

    if args.play:
        # IsKeyDown true/false simulates press and release, useful for "hold" modes.
        print(f"-> playMeme {args.play} (down)")
        await ws.send(request("playMeme", {"FileName": args.play, "IsKeyDown": True}))
        await drain(ws, 0.5, log)
        print(f"-> playMeme {args.play} (up)")
        await ws.send(request("playMeme", {"FileName": args.play, "IsKeyDown": False}))
        # Keep listening to see whether playback start/end events arrive.
        await drain(ws, args.listen, log)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="localhost")
    parser.add_argument("--port", type=int, default=59129)
    parser.add_argument("--key", default=load_client_key(), help="clientKey provided by Voicemod")
    parser.add_argument("--play", help="FileName of a meme to play as a test")
    parser.add_argument("--listen", type=float, default=10, help="seconds to listen after playMeme")
    parser.add_argument("--out", default="voicemod_probe_out")
    args = parser.parse_args()
    if not args.key:
        parser.error(f"missing clientKey: use --key or set VOICEMOD_CLIENT_KEY in {ENV_FILE}")
    asyncio.run(main(args))
