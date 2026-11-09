#!/usr/bin/env python3
"""Run AccuracyCoin.nes in headless Mesen2 and capture an oracle-evidence dump.

Reproducibility chain (metroid-decomp tools/oracle/accuracycoin):
  1. ROM:  analysis/accuracycoin/AccuracyCoin.nes
           (fetched from doublegate/rustynes tests/roms/accuracycoin, v3.6.2,
            sha256=9332c3735dd7dea2f4711e670d6478d8b0085edeacdd4f9f300fb04a8bbe3f88,
            md5=6cab1c9cc644e51ff4830fbe779f66d9 = RUNTIME.md-attested copy of
            upstream 100thCoin/AccuracyCoin v3.6.2)
  2. Mesen2 binary: tools/mesen2/Mesen (2.1.1). Runs under a throwaway Xvfb display
     with an isolated HOME (settings.json pre-seeded so the first-run wizard does not
     silently exit; see findings doc for the full pitfall list).
  3. Lua:  accuracycoin_dump.lua (this dir) — dumps CHR/NT/RAM every 1800 frames.
  4. Decode: decode_accuracycoin.py <dump.log> AccuracyCoin.asm

Holds analysis/.scenario.lock (fail-fast) — never run concurrently with the L4
harness; Mesen2 is single-instance per user and scenario_run.py's orphan sweep
kills stray Mesen processes.
"""
import argparse
import fcntl
import os
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rom", default="analysis/accuracycoin/AccuracyCoin.nes")
    ap.add_argument("--lua", default=os.path.join(os.path.dirname(__file__), "accuracycoin_dump.lua"))
    ap.add_argument("--out", default="analysis/accuracycoin/accuracycoin_dump.log")
    ap.add_argument("--mesen", default="tools/mesen2/Mesen")
    ap.add_argument("--home", default=os.path.expanduser("~/.config/acc-home"))
    ap.add_argument("--display", default=":121")
    ap.add_argument("--timeout", type=int, default=420)
    args = ap.parse_args()

    lock_path = os.path.join(ROOT, "analysis", ".scenario.lock")
    f = open(lock_path, "a+")
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        sys.exit("scenario lock held by a sibling — retry later")
    print("lock ok", time.strftime("%H:%M:%S"), flush=True)

    log_path = os.path.join(ROOT, args.out)
    xv = subprocess.Popen(["Xvfb", args.display, "-screen", "0", "1024x768x24", "-nolisten", "tcp"])
    time.sleep(1.5)
    env = dict(os.environ, HOME=args.home, DISPLAY=args.display)
    mp = subprocess.Popen(
        [args.mesen, args.rom, args.lua, "/DoNotSaveSettings"],
        stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT, env=env, cwd=ROOT)
    done = False
    for i in range(args.timeout):
        time.sleep(1)
        try:
            if "ENDDUMP" in open(log_path).read():
                done = True
                break
        except FileNotFoundError:
            pass
        if mp.poll() is not None:
            print(f"mesen exited rc={mp.returncode} at {i}s", flush=True)
            break
    print("dump complete" if done else "timeout — partial dump kept", flush=True)
    mp.terminate(); time.sleep(1); xv.terminate()
    fcntl.flock(f, fcntl.LOCK_UN)
    print("released", flush=True)


if __name__ == "__main__":
    main()
