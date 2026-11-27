"""Command line of the oracle harness."""

import argparse
import datetime
import hashlib
import os
import sys
from typing import List, Optional

from . import TOOL_VERSION
from .backend import AttachError, Backend, ProcessGone
from .buildid import build_id, install_root_from_exe
from .capture import build_sidecar, run_capture, write_sidecar
from .config import ConfigError, default_config_path, load_config
from .reader import FieldReader, missing_modules

EXIT_OK = 0
EXIT_USAGE = 2
EXIT_ATTACH = 3
EXIT_NO_DATA = 4

DEFAULT_PROCESS = "Sam2.exe"


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="oracle.py",
        description=(
            "Read-only measurement of the running original game: reads numbers from its memory "
            "through a local, git-ignored config and writes a CSV (numbers only) plus a JSON sidecar."
        ),
    )
    p.add_argument("--version", action="version", version="ss2-oracle " + TOOL_VERSION)
    p.add_argument("--config", help="target map (default: %%LOCALAPPDATA%%/ss2rs-oracle/targets.json)")
    target = p.add_argument_group("which process")
    target.add_argument("--pid", type=int, help="attach to this process id")
    target.add_argument("--process", metavar="NAME", help="attach to the running process with this exe name (default %s)" % DEFAULT_PROCESS)
    target.add_argument("--wait-for-process", metavar="NAME", help="wait until a process with this exe name runs, then attach")
    target.add_argument("--wait-timeout", type=float, default=300.0, metavar="SECONDS", help="how long --wait-for-process waits (default 300)")
    mode = p.add_argument_group("what to do (default: capture)")
    mode.add_argument("--validate-config", action="store_true", help="check the config file and exit; needs no process")
    mode.add_argument("--list-modules", action="store_true", help="print the process's modules (name, base, size) and exit")
    mode.add_argument("--dump-fields", action="store_true", help="read every configured field once, print name = value, and exit")
    cap = p.add_argument_group("capture")
    cap.add_argument("--scenario", help="a short name for what is being measured (goes into the sidecar)")
    cap.add_argument("--out", metavar="FILE.csv", help="CSV to write; the sidecar is FILE.csv.json")
    cap.add_argument("--rate", type=float, default=30.0, metavar="HZ", help="samples per second (default 30)")
    cap.add_argument("--duration", type=float, metavar="SECONDS", help="stop after this long (default: until the process exits or Ctrl-C)")
    cap.add_argument("--samples", type=int, metavar="N", help="stop after N samples")
    cap.add_argument("--install", metavar="DIR", help="the game install, for the build id (default: derived from the process's exe path)")
    cap.add_argument("--force", action="store_true", help="overwrite an existing output file")
    return p


def _attach(args) -> Backend:
    from .winproc import WindowsProcess, find_processes, wait_for_process

    if args.pid is not None:
        return WindowsProcess(args.pid)
    if args.wait_for_process:
        print("waiting for %s ..." % args.wait_for_process, file=sys.stderr)
        return WindowsProcess(wait_for_process(args.wait_for_process, args.wait_timeout))
    name = args.process or DEFAULT_PROCESS
    pids = find_processes(name)
    if not pids:
        raise AttachError("no process called %s is running (use --wait-for-process to wait)" % name)
    if len(pids) > 1:
        raise AttachError("%d processes are called %s; pass --pid" % (len(pids), name))
    return WindowsProcess(pids[0])


def _sha256_of(path: str) -> Optional[str]:
    try:
        with open(path, "rb") as handle:
            return hashlib.sha256(handle.read()).hexdigest()
    except OSError:
        return None


def main(argv: Optional[List[str]] = None, attach=None) -> int:
    """Entry point. ``attach`` lets a test supply a backend instead of a real process."""
    args = build_parser().parse_args(argv)
    attach = attach or _attach
    config_path = args.config or default_config_path()

    needs_config = not args.list_modules
    config = None
    if needs_config:
        try:
            config = load_config(config_path)
        except ConfigError as exc:
            print("config error: %s" % exc, file=sys.stderr)
            return EXIT_USAGE
        if args.validate_config:
            print("config ok: %d columns" % len(config.column_names()))
            for column in config.column_names():
                print("  " + column)
            return EXIT_OK

    capturing = not (args.list_modules or args.dump_fields)
    if capturing:
        if not args.out or not args.scenario:
            print("a capture needs --out FILE.csv and --scenario NAME", file=sys.stderr)
            return EXIT_USAGE
        if not (0 < args.rate <= 1000):
            print("--rate must be between 0 and 1000", file=sys.stderr)
            return EXIT_USAGE
        if args.duration is not None and args.duration <= 0:
            print("--duration must be positive", file=sys.stderr)
            return EXIT_USAGE
        if args.samples is not None and args.samples <= 0:
            print("--samples must be positive", file=sys.stderr)
            return EXIT_USAGE
        if os.path.exists(args.out) and not args.force:
            print("%s exists; pass --force to overwrite" % args.out, file=sys.stderr)
            return EXIT_USAGE

    try:
        backend = attach(args)
    except AttachError as exc:
        print("cannot attach: %s" % exc, file=sys.stderr)
        return EXIT_ATTACH

    try:
        print(
            "attached: pid %s, %d-bit pointers" % (backend.pid(), backend.pointer_size * 8),
            file=sys.stderr,
        )
        if args.list_modules:
            for m in sorted(backend.list_modules(), key=lambda m: m.base):
                print("%-28s base 0x%08X  size 0x%X" % (m.name, m.base, m.size))
            return EXIT_OK

        try:
            reader = FieldReader(backend, config)
        except ValueError as exc:
            print("config error: %s" % exc, file=sys.stderr)
            return EXIT_USAGE
        absent = missing_modules(config, backend.list_modules())
        if absent:
            print("note: not loaded yet: %s (their fields read as empty until they load)" % ", ".join(absent), file=sys.stderr)

        if args.dump_fields:
            sample = reader.sample()
            for (name, ftype), cell in zip(reader.describe(), sample.cells):
                print("%-40s %-4s %s" % (name, ftype, cell if cell != "" else "(invalid)"))
            return EXIT_OK if sample.valid else EXIT_NO_DATA

        install = args.install
        if install is None and backend.image_path():
            install = install_root_from_exe(backend.image_path())
        build = build_id(install) if install and os.path.isdir(install) else None
        if build is None:
            print("note: no install folder, so no build id in the sidecar (use --install)", file=sys.stderr)
        started = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        folder = os.path.dirname(os.path.abspath(args.out))
        os.makedirs(folder, exist_ok=True)
        with open(args.out, "w", encoding="utf-8", newline="") as out:
            stats = run_capture(reader, out, args.rate, args.duration, args.samples)
        sidecar = build_sidecar(
            args.scenario,
            args.rate,
            os.path.basename(backend.image_path() or "") or "unknown",
            backend.pid(),
            backend.pointer_size,
            reader.describe(),
            stats,
            build,
            _sha256_of(config_path),
            started,
        )
        write_sidecar(args.out + ".json", sidecar)
        print(
            "%d samples, %d missed slots, ended: %s" % (stats.samples, stats.missed_slots, stats.ended),
            file=sys.stderr,
        )
        return EXIT_OK if stats.samples else EXIT_NO_DATA
    except ProcessGone:
        print("the process exited", file=sys.stderr)
        return EXIT_NO_DATA
    finally:
        backend.close()
