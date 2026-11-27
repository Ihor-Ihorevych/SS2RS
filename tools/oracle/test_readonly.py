"""The harness must not be able to change the process it measures. This test greps its own
sources for the Windows calls that write, inject, suspend, debug or send input."""

import os
import unittest

FORBIDDEN = [
    "WriteProcessMemory",
    "VirtualAllocEx",
    "VirtualProtectEx",
    "CreateRemoteThread",
    "NtWriteVirtualMemory",
    "DebugActiveProcess",
    "SuspendThread",
    "ResumeThread",
    "SetThreadContext",
    "SendInput",
    "PostMessage",
    "SendMessage",
    "keybd_event",
    "mouse_event",
    "LoadLibrary",
    "TerminateProcess",
    "PROCESS_VM_WRITE",
    "PROCESS_VM_OPERATION",
    "PROCESS_ALL_ACCESS",
    "PROCESS_TERMINATE",
    "ntdll",
]


class ReadOnlyTests(unittest.TestCase):
    def sources(self):
        root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ss2_oracle")
        for name in sorted(os.listdir(root)):
            if name.endswith(".py"):
                with open(os.path.join(root, name), encoding="utf-8") as f:
                    yield name, f.read()

    def test_no_write_inject_or_input_api_is_named(self):
        for name, text in self.sources():
            for word in FORBIDDEN:
                self.assertNotIn(word, text, "%s mentions %s" % (name, word))

    def test_process_is_opened_with_read_and_query_rights_only(self):
        text = dict(self.sources())["winproc.py"]
        self.assertIn("PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, False", text)
        self.assertEqual(text.count("OpenProcess("), 1)

    def test_backend_interface_has_no_write_method(self):
        from ss2_oracle.backend import Backend

        names = [n for n in dir(Backend) if not n.startswith("_")]
        for n in names:
            self.assertNotIn("write", n.lower())


if __name__ == "__main__":
    unittest.main()
