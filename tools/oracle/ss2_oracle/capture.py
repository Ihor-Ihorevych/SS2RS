"""The timed capture loop: fixed-rate sampling, CSV rows, jitter accounting, the JSON sidecar."""

import csv
import json
import math
import time
from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional

from . import TOOL_VERSION
from .backend import ProcessGone
from .reader import FieldReader

FIXED_COLUMNS = ["sample", "t_wall_s", "jitter_s", "read_s", "valid", "n_invalid"]


@dataclass
class CaptureStats:
    samples: int = 0
    missed_slots: int = 0
    invalid_rows: int = 0
    ended: str = "not started"
    jitter: List[float] = field(default_factory=list)
    intervals: List[float] = field(default_factory=list)
    read_times: List[float] = field(default_factory=list)
    duration_s: float = 0.0

    def summary(self) -> Dict[str, object]:
        def percentile(values: List[float], q: float) -> Optional[float]:
            if not values:
                return None
            ordered = sorted(values)
            return ordered[min(len(ordered) - 1, int(math.ceil(q * len(ordered))) - 1)]

        return {
            "samples": self.samples,
            "missed_slots": self.missed_slots,
            "rows_with_invalid_cells": self.invalid_rows,
            "ended": self.ended,
            "duration_s": round(self.duration_s, 6),
            "jitter_s": {
                "mean": _mean(self.jitter),
                "p95": percentile(self.jitter, 0.95),
                "max": max(self.jitter) if self.jitter else None,
            },
            "interval_s": {
                "mean": _mean(self.intervals),
                "min": min(self.intervals) if self.intervals else None,
                "max": max(self.intervals) if self.intervals else None,
            },
            "read_s": {
                "mean": _mean(self.read_times),
                "max": max(self.read_times) if self.read_times else None,
            },
        }


def _mean(values: List[float]) -> Optional[float]:
    return (sum(values) / len(values)) if values else None


def _wait_until(target: float, clock: Callable[[], float], sleep: Callable[[float], None]) -> None:
    """Sleep until ``target`` on ``clock``. Python 3.11+ sleeps on a high-resolution timer on
    Windows, so one sleep per slot lands within about a millisecond; the jitter column records it."""
    while True:
        remaining = target - clock()
        if remaining <= 0:
            return
        sleep(remaining)


def run_capture(
    reader: FieldReader,
    out,
    rate_hz: float,
    duration_s: Optional[float] = None,
    max_samples: Optional[int] = None,
    clock: Callable[[], float] = time.perf_counter,
    sleep: Callable[[float], None] = time.sleep,
    stop: Optional[Callable[[], bool]] = None,
) -> CaptureStats:
    """Sample ``reader`` at ``rate_hz`` into the text stream ``out`` as CSV.

    Slot ``k`` is at ``t0 + k / rate``. A sample that ends after the next slot has started makes the
    skipped slots count as ``missed_slots`` (the loop does not burst to catch up). ``jitter_s`` is
    how late a sample began compared with its own slot; ``t_wall_s`` is its real start time, so the
    true spacing is always recoverable from the file.
    """
    if not (rate_hz > 0 and math.isfinite(rate_hz)):
        raise ValueError("rate must be a positive number of samples per second")
    period = 1.0 / rate_hz
    writer = csv.writer(out, lineterminator="\n")
    columns = reader.config.column_names()
    writer.writerow(FIXED_COLUMNS + columns)

    stats = CaptureStats()
    t0 = clock()
    slot = 0
    previous_start: Optional[float] = None
    try:
        while True:
            slot_time = slot * period
            if duration_s is not None and slot_time >= duration_s:
                stats.ended = "duration reached"
                break
            if max_samples is not None and stats.samples >= max_samples:
                stats.ended = "sample count reached"
                break
            if stop is not None and stop():
                stats.ended = "stopped"
                break
            _wait_until(t0 + slot_time, clock, sleep)
            start = clock()
            try:
                result = reader.sample()
            except ProcessGone:
                stats.ended = "process exited"
                break
            end = clock()
            rel = start - t0
            jitter = rel - slot_time
            writer.writerow(
                [
                    stats.samples,
                    "%.6f" % rel,
                    "%.6f" % jitter,
                    "%.6f" % (end - start),
                    1 if result.valid else 0,
                    result.n_invalid,
                ]
                + result.cells
            )
            stats.samples += 1
            if not result.valid:
                stats.invalid_rows += 1
            stats.jitter.append(jitter)
            stats.read_times.append(end - start)
            if previous_start is not None:
                stats.intervals.append(start - previous_start)
            previous_start = start
            if stats.samples % 30 == 0:
                out.flush()
            # next slot: the first one that has not started yet
            next_slot = slot + 1
            late = int(math.floor((end - t0) / period)) + 1
            if late > next_slot:
                stats.missed_slots += late - next_slot
                next_slot = late
            slot = next_slot
    except KeyboardInterrupt:
        stats.ended = "interrupted"
    stats.duration_s = clock() - t0
    out.flush()
    return stats


def build_sidecar(
    scenario: str,
    rate_hz: float,
    process_name: str,
    pid: Optional[int],
    pointer_size: int,
    columns: List[tuple],
    stats: CaptureStats,
    build: Optional[dict],
    config_sha256: Optional[str],
    started_utc: str,
) -> dict:
    """The metadata file written next to the CSV. Contains no addresses or offsets."""
    return {
        "tool": "ss2-oracle",
        "tool_version": TOOL_VERSION,
        "scenario": scenario,
        "started_utc": started_utc,
        "rate_hz": rate_hz,
        "process": {"name": process_name, "pid": pid, "pointer_size": pointer_size},
        "install_build": build,
        "config_sha256": config_sha256,
        "columns": [{"name": name, "type": ftype} for name, ftype in columns],
        "fixed_columns": FIXED_COLUMNS,
        "capture": stats.summary(),
    }


def write_sidecar(path: str, sidecar: dict) -> None:
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(sidecar, handle, indent=2, sort_keys=False)
        handle.write("\n")
