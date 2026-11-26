"""Hand-written mutation check of the harness's core logic.

For each entry the exact text OLD in FILE is replaced by NEW, the whole test suite is run, and the
file is restored. A mutant is KILLED when the suite fails and SURVIVED when it still passes.

    python -I tools/oracle/mutate.py            # needs Windows for the real-process tests
    python -I tools/oracle/mutate.py --no-self  # skip them (the fake-process tests only)

Exit status 0 when every mutant was killed, 1 otherwise. Survivors are printed; some can be
equivalent mutants (the changed code behaves the same); the list below says which, if any.
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.join(HERE, "ss2_oracle")

# (file, old text, new text, what the mutant breaks)
MUTATIONS = [
    # --- pointer chains -------------------------------------------------------------------
    ("reader.py", "if not pointer:", "if pointer is None:", "null pointers are followed"),
    ("reader.py", "address = pointer + offsets[position]", "address = pointer", "chain step offset dropped"),
    ("reader.py", "address = base + offsets[0]", "address = offsets[0]", "module base not added"),
    ("reader.py", "for position in range(1, len(offsets)):", "for position in range(2, len(offsets)):", "first dereference skipped"),
    ("reader.py", "offsets[spec.repeat.at] += index * spec.repeat.stride", "offsets[spec.repeat.at] += index", "repeat stride ignored"),
    ("reader.py", "if address >= self._limit:\n                return None\n            if spec.handles", "if address > self._limit * 2:\n                return None\n            if spec.handles", "pointer-width overflow in the chain not caught"),
    ("reader.py", "        if address >= self._limit:\n            return None\n        return address", "        if address > self._limit * 2:\n            return None\n        return address", "final address overflow not caught"),
    ("reader.py", "if len(data) != size:", "if len(data) > size:", "short read accepted"),
    ("reader.py", "self._cache = {}\n        cells", "pass\n        cells", "per-sample cache never cleared (stale values)"),
    ("reader.py", "if key in self._cache:", "if False:", "shared pointers read once per sample"),
    ("reader.py", "if now - self._last_lookup < self._retry:", "if False:", "module lookup not rate limited"),
    ("reader.py", "self._bases.setdefault(module.name.lower(), module.base)", "self._bases.setdefault(module.name, module.base)", "module names case-sensitive"),
    ("reader.py", "if not self.backend.is_alive():\n                raise ProcessGone", "if False:\n                raise ProcessGone", "process exit not noticed on failed read"),
    ("reader.py", "if config.pointer_size is not None and config.pointer_size != self.pointer_size:", "if False:", "pointer size mismatch not refused"),
    ("reader.py", "if cell == \"\":\n                    invalid += 1", "if False:\n                    invalid += 1", "invalid cells not counted"),
    # --- number formatting ----------------------------------------------------------------
    ("reader.py", "for digits in range(1, 10):", "for digits in range(1, 4):", "f32 text too short to round-trip"),
    ("reader.py", "return _format_double(float(value) * scale)", "return _format_double(float(value))", "scale ignored"),
    ("reader.py", "return str(int(value))", "return str(float(value))", "integers written as floats"),
    ("config.py", '"i64": ("<q", 8, False)', '"i64": ("<Q", 8, False)', "i64 decoded unsigned"),
    ("config.py", '"f32": ("<f", 4, True)', '"f32": (">f", 4, True)', "f32 big-endian"),
    ("config.py", '"i16": ("<h", 2, False)', '"i16": ("<H", 2, False)', "i16 decoded unsigned"),
    ("config.py", '"u32": ("<I", 4, False)', '"u32": ("<i", 4, False)', "u32 decoded signed"),
    # --- config validation ----------------------------------------------------------------
    ("config.py", "if value < 0:", "if value < -1:", "-1 offset accepted"),
    ("config.py", "if value > MAX_OFFSET:", "if value >= MAX_OFFSET:", "largest offset rejected"),
    ("config.py", "if ftype not in TYPES:", "if False:", "unknown types accepted"),
    ("config.py", "if name in RESERVED_NAMES:", "if False:", "reserved column names accepted"),
    ("config.py", "if not 0 <= r[\"at\"] < len(offsets):", "if not 0 <= r[\"at\"] <= len(offsets):", "repeat.at one past the chain accepted"),
    ("config.py", "if len(offsets_raw) > MAX_CHAIN:", "if len(offsets_raw) > MAX_CHAIN + 1:", "chain one too long accepted"),
    ("config.py", "if not MODULE_RE.match(module):", "if False:", "module paths accepted"),
    ("config.py", "raise ConfigError(\"duplicate key %r in the config file\" % key)", "pass", "duplicate JSON keys accepted"),
    ("config.py", "if doc.get(\"format\") != FORMAT_VERSION:", "if False:", "format version not checked"),
    # --- capture loop and jitter accounting -----------------------------------------------
    ("capture.py", "late = int(math.floor((end - t0) / period)) + 1", "late = int(math.floor((end - t0) / period))", "slow samples burst to catch up"),
    ("capture.py", "stats.missed_slots += late - next_slot", "stats.missed_slots += 0", "missed slots not counted"),
    ("capture.py", "jitter = rel - slot_time", "jitter = rel", "jitter measured from t0 not from the slot"),
    ("capture.py", "if duration_s is not None and slot_time >= duration_s:", "if duration_s is not None and slot_time > duration_s:", "one sample too many at the end of the duration"),
    ("capture.py", "1 if result.valid else 0,", "1,", "valid column always 1"),
    ("capture.py", "stats.jitter.append(jitter)", "stats.jitter.append(0.0)", "jitter statistics always zero"),
    ("capture.py", "if previous_start is not None:", "if False:", "interval statistics lost"),
    ("capture.py", "except ProcessGone:\n                stats.ended = \"process exited\"\n                break", "except ZeroDivisionError:\n                stats.ended = \"process exited\"\n                break", "process exit crashes the capture"),
    ("capture.py", "if not (rate_hz > 0 and math.isfinite(rate_hz)):", "if False:", "bad rate accepted"),
    ("capture.py", "writer.writerow(FIXED_COLUMNS + columns)", "writer.writerow(columns)", "header lacks the fixed columns"),
    ("capture.py", "\"%.6f\" % rel,", "\"%.3f\" % rel,", "sample times rounded to milliseconds"),
    # --- command line ---------------------------------------------------------------------
    ("cli.py", "if os.path.exists(args.out) and not args.force:", "if False:", "existing output overwritten silently"),
    ("cli.py", "return EXIT_OK if sample.valid else EXIT_NO_DATA", "return EXIT_OK", "--dump-fields hides invalid fields in the exit status"),
    ("buildid.py", "digest.update((\"%s:%d\\n\" % (name, size)).encode())", "pass", "patch archives not part of the build id"),
    ("buildid.py", "return struct.unpack_from(\"<I\", header, 8)[0]", "return struct.unpack_from(\"<I\", header, 4)[0]", "PE time stamp read from the wrong place"),
    # --- handle steps ---------------------------------------------------------------------
    ("reader.py", "if handle == 0:", "if False:", "handle 0 resolved through the table"),
    ("reader.py", "index = handle & ((1 << table.index_bits) - 1)", "index = handle & 0xFFFF", "handle index mask fixed"),
    ("reader.py", "generation = handle >> table.index_bits", "generation = handle >> 24", "generation shift fixed"),
    ("reader.py", "if index >= int.from_bytes(count_raw, \"little\"):", "if index > int.from_bytes(count_raw, \"little\"):", "index equal to the count accepted"),
    ("reader.py", "if int.from_bytes(gen_raw, \"little\") != generation:", "if False:", "stale generation accepted"),
    ("reader.py", "record = records + index * table.record_size", "record = records + index * 16", "record size fixed"),
    ("reader.py", "if records == 0:", "if False:", "missing table not detected"),
    ("reader.py", "gen_raw = self._read(record + table.generation_at, 4)", "gen_raw = self._read(record + 0, 4)", "generation offset ignored"),
    ("reader.py", "obj_raw = self._read(record + table.object_at, self.pointer_size)", "obj_raw = self._read(record + 8, self.pointer_size)", "object offset ignored"),
    ("reader.py", "if spec.handles[position]:", "if False:", "handle steps read as pointers"),
    ("config.py", "if generation_at + 4 > record_size or object_at + 4 > record_size:", "if False:", "fields outside the record accepted"),
    ("config.py", "if position == 0:", "if False:", "first step may be a handle"),
    ("config.py", "if any(spec.handles):", "if False:", "handle steps without a table accepted"),
    ("config.py", "if set(item) != {\"offset\", \"handle\"} or item[\"handle\"] is not True:", "if False:", "malformed step objects accepted"),
    # --- the real Windows backend (killed only by the real-process tests) -------------------
    ("winproc.py", "self.pointer_size = 4 if wow64.value else struct.calcsize(\"P\")", "self.pointer_size = 4", "64-bit target read with 4-byte pointers"),
    ("winproc.py", "if not ok or done.value != size:", "if False:", "failed reads return garbage"),
    ("winproc.py", "return code.value == STILL_ACTIVE", "return True", "process exit never noticed"),
    ("winproc.py", "if entry.szExeFile.lower() == name.lower():", "if entry.szExeFile == name:", "process name match case-sensitive"),
]


def run_suite(no_self: bool) -> bool:
    env = dict(os.environ)
    if not no_self:
        env["SS2_ORACLE_SELF_TEST"] = "1"
    else:
        env.pop("SS2_ORACLE_SELF_TEST", None)
    try:
        proc = subprocess.run(
            [sys.executable, "-I", "-m", "unittest", "discover", "-s", HERE],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=60,
        )
    except subprocess.TimeoutExpired:
        return False  # a mutant that hangs the suite is detected too
    return proc.returncode == 0


def main() -> int:
    no_self = "--no-self" in sys.argv
    if not run_suite(no_self):
        print("the unmutated suite already fails; fix that first")
        return 2
    bad = [(f, w) for f, old, _new, w in MUTATIONS if open(os.path.join(PKG, f), encoding="utf-8", newline="").read().count(old) != 1]
    if bad:
        for f, w in bad:
            print("BAD MUTATION (its text does not occur exactly once): %s: %s" % (f, w))
        return 2
    killed, survived = [], []
    for filename, old, new, what in MUTATIONS:
        path = os.path.join(PKG, filename)
        with open(path, encoding="utf-8", newline="") as handle:
            original = handle.read()
        if original.count(old) != 1:
            print("BAD MUTATION (text occurs %d times): %s: %s" % (original.count(old), filename, what))
            return 2
        try:
            with open(path, "w", encoding="utf-8", newline="") as handle:
                handle.write(original.replace(old, new))
            passed = run_suite(no_self)
        finally:
            with open(path, "w", encoding="utf-8", newline="") as handle:
                handle.write(original)
        (survived if passed else killed).append((filename, what))
        print("%-9s %-12s %s" % ("SURVIVED" if passed else "killed", filename, what), flush=True)
    print("\n%d mutants: %d killed, %d survived" % (len(MUTATIONS), len(killed), len(survived)))
    for filename, what in survived:
        print("  survived: %s: %s" % (filename, what))
    return 0 if not survived else 1


if __name__ == "__main__":
    sys.exit(main())
