"""One real-process test: the harness attaches to THIS Python process and reads known buffers
through the real ReadProcessMemory path (module lookup, pointer chains, type decoding, the
capture loop). Skipped unless SS2_ORACLE_SELF_TEST=1, because it needs Windows.

    set SS2_ORACLE_SELF_TEST=1
    python -I -m unittest discover -s tools/oracle -v
"""

import ctypes
import io
import json
import os
import struct
import sys
import unittest

ENABLED = os.environ.get("SS2_ORACLE_SELF_TEST") == "1" and sys.platform == "win32"


@unittest.skipUnless(ENABLED, "set SS2_ORACLE_SELF_TEST=1 on Windows to run the real ReadProcessMemory test")
class SelfProcessTests(unittest.TestCase):
    def setUp(self):
        from ss2_oracle.winproc import WindowsProcess

        self.proc = WindowsProcess(os.getpid())
        self.addCleanup(self.proc.close)

    def test_pointer_size_and_liveness(self):
        self.assertEqual(self.proc.pointer_size, struct.calcsize("P"))
        self.assertFalse(self.proc.is_wow64 and self.proc.pointer_size != 4)
        self.assertTrue(self.proc.is_alive())
        self.assertEqual(self.proc.pid(), os.getpid())

    def test_modules_include_the_python_executable_and_base_matches_the_loader(self):
        names = {m.name.lower(): m for m in self.proc.list_modules()}
        exe = os.path.basename(sys.executable).lower()
        self.assertIn(exe, names)
        self.assertIn("kernel32.dll", names)
        # the base the snapshot reports is the one the loader gave kernel32
        k32 = ctypes.WinDLL("kernel32")
        k32.GetModuleHandleW.restype = ctypes.c_void_p
        k32.GetModuleHandleW.argtypes = [ctypes.c_wchar_p]
        self.assertEqual(names["kernel32.dll"].base, k32.GetModuleHandleW("kernel32.dll"))

    def test_image_path_is_this_executable(self):
        self.assertEqual(os.path.normcase(self.proc.image_path()), os.path.normcase(os.path.normpath(sys.executable)))

    def test_reads_a_known_buffer_and_fails_cleanly_on_unmapped_memory(self):
        from ss2_oracle.backend import ReadError

        buf = ctypes.create_string_buffer(b"\x01\x02\x03\x04\x05\x06\x07\x08", 8)
        self.assertEqual(self.proc.read(ctypes.addressof(buf), 8), b"\x01\x02\x03\x04\x05\x06\x07\x08")
        with self.assertRaises(ReadError):
            self.proc.read(0x10, 4)  # the null page
        self.assertTrue(self.proc.is_alive())

    def alloc_near(self, base):
        """A fresh committed page within 2 GB above ``base`` (so a config offset can reach it)."""
        k = ctypes.WinDLL("kernel32", use_last_error=True)
        k.VirtualAlloc.restype = ctypes.c_void_p
        k.VirtualAlloc.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint32, ctypes.c_uint32]
        k.VirtualFree.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint32]
        for step in range(0x100000, 0x70000000, 0x100000):
            address = k.VirtualAlloc(base + step, 0x1000, 0x3000, 0x04)  # COMMIT|RESERVE, READWRITE
            if address:
                self.addCleanup(k.VirtualFree, address, 0, 0x8000)  # RELEASE
                return address
        self.skipTest("no free page near the module on this run")

    def test_pointer_chain_through_the_real_path(self):
        from ss2_oracle.config import parse_config
        from ss2_oracle.reader import FieldReader

        class Inner(ctypes.Structure):
            _fields_ = [("pad", ctypes.c_uint32), ("speed", ctypes.c_float), ("tick", ctypes.c_int64)]

        inner = Inner(0, 12.5, -(2**40))
        module = "kernel32.dll"
        base = self.proc.module_base(module)
        page = self.alloc_near(base)
        slot = page  # a "global" in the module's neighbourhood: holds a pointer to inner
        psize = ctypes.sizeof(ctypes.c_void_p)
        ctypes.memmove(slot, ctypes.byref(ctypes.c_void_p(ctypes.addressof(inner))), psize)
        # a second slot that holds a null pointer
        ctypes.memmove(slot + 16, ctypes.byref(ctypes.c_void_p(0)), psize)
        offset = slot - base
        self.assertLess(offset, 2**32)
        cfg = parse_config(
            json.dumps(
                {
                    "format": 1,
                    "fields": {
                        "speed": {"module": module, "offsets": [offset, 4], "type": "f32"},
                        "tick": {"module": module, "offsets": [offset, 8], "type": "i64"},
                        "dead": {"module": module, "offsets": [offset + 16, 4], "type": "f32"},
                        "arr": {"module": module, "offsets": [offset, 4], "type": "f32", "repeat": {"count": 2, "stride": 16, "at": 0}},
                    },
                }
            )
        )
        sample = FieldReader(self.proc, cfg).sample()
        self.assertEqual(sample.cells, ["12.5", str(-(2**40)), "", "12.5", ""])
        self.assertEqual(sample.n_invalid, 2)

    def test_capture_loop_on_the_real_process(self):
        import time

        from ss2_oracle.capture import run_capture
        from ss2_oracle.config import parse_config
        from ss2_oracle.reader import FieldReader

        module = "kernel32.dll"
        base = self.proc.module_base(module)
        page = self.alloc_near(base)
        ctypes.memmove(page, struct.pack("<I", 0xCAFE), 4)
        cfg = parse_config(
            json.dumps({"format": 1, "fields": {"v": {"module": module, "offsets": [page - base], "type": "u32"}}})
        )
        out = io.StringIO()
        start = time.perf_counter()
        stats = run_capture(FieldReader(self.proc, cfg), out, 50, duration_s=0.5)
        self.assertGreaterEqual(stats.samples, 20)
        self.assertLess(time.perf_counter() - start, 2.0)
        lines = out.getvalue().splitlines()[1:]
        self.assertTrue(all(line.endswith(",1,0,%d" % 0xCAFE) for line in lines))

    def test_the_real_cli_attaches_by_pid_and_lists_modules(self):
        import contextlib

        from ss2_oracle import cli

        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cli.main(["--pid", str(os.getpid()), "--list-modules"])
        self.assertEqual(code, 0, err.getvalue())
        self.assertIn("kernel32.dll", out.getvalue().lower())

    def test_full_command_line_capture_on_the_real_process(self):
        import contextlib
        import tempfile

        from ss2_oracle import cli

        module = "kernel32.dll"
        base = self.proc.module_base(module)
        page = self.alloc_near(base)
        ctypes.memmove(page, struct.pack("<f", 100.0), 4)
        with tempfile.TemporaryDirectory() as folder:
            config = os.path.join(folder, "targets.json")
            with open(config, "w") as handle:
                json.dump({"format": 1, "fields": {"hp": {"module": module, "offsets": [page - base], "type": "f32"}}}, handle)
            out_csv = os.path.join(folder, "out", "run.csv")
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = cli.main(["--config", config, "--pid", str(os.getpid()), "--scenario", "selftest", "--out", out_csv, "--rate", "100", "--duration", "0.3", "--install", folder])
            self.assertEqual(code, 0, err.getvalue())
            with open(out_csv) as handle:
                lines = handle.read().splitlines()
            self.assertEqual(lines[0], "sample,t_wall_s,jitter_s,read_s,valid,n_invalid,hp")
            self.assertGreaterEqual(len(lines) - 1, 20)
            self.assertTrue(all(line.endswith(",1,0,100.0") for line in lines[1:]))
            with open(out_csv + ".json") as handle:
                side = json.load(handle)
            self.assertEqual(side["scenario"], "selftest")
            self.assertEqual(side["process"]["pointer_size"], struct.calcsize("P"))
            self.assertEqual(side["capture"]["ended"], "duration reached")

    def test_wait_for_process_finds_this_process_by_name(self):
        from ss2_oracle.winproc import find_processes, wait_for_process

        exe = os.path.basename(sys.executable)
        self.assertIn(os.getpid(), find_processes(exe))
        self.assertIn(os.getpid(), find_processes(exe.upper()))
        # several python processes may run; wait_for_process refuses an ambiguous name
        if len(find_processes(exe)) == 1:
            self.assertEqual(wait_for_process(exe, 2.0), os.getpid())

    def test_timeout_when_no_such_process(self):
        from ss2_oracle.backend import AttachError
        from ss2_oracle.winproc import wait_for_process

        with self.assertRaises(AttachError):
            wait_for_process("definitely-not-running-xyz.exe", 0.3, poll_s=0.05)

    def test_exit_of_a_real_child_process_is_seen(self):
        import subprocess

        from ss2_oracle.backend import ProcessGone
        from ss2_oracle.winproc import WindowsProcess

        child = subprocess.Popen([sys.executable, "-I", "-c", "import time; time.sleep(30)"])
        try:
            proc = WindowsProcess(child.pid)
            self.assertTrue(proc.is_alive())
            import time

            deadline = time.monotonic() + 10
            modules = proc.list_modules()
            while not modules and time.monotonic() < deadline:  # the child is still starting
                time.sleep(0.05)
                modules = proc.list_modules()
            base = modules[0].base
            self.assertEqual(proc.read(base, 2), b"MZ")
            child.kill()
            child.wait()
            self.assertFalse(proc.is_alive())
            with self.assertRaises(ProcessGone):
                proc.read(base, 2)
            proc.close()
        finally:
            if child.poll() is None:
                child.kill()


if __name__ == "__main__":
    unittest.main()
