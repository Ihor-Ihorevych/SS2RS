"""Windows backend: ReadProcessMemory through ctypes. Read-only by construction.

The only access right requested is PROCESS_VM_READ | PROCESS_QUERY_INFORMATION. No function
that writes, suspends, injects or debugs is declared in this file.
"""

import ctypes
import os
import struct
import sys
import time
from ctypes import wintypes
from typing import List, Optional

from .backend import AttachError, Backend, ModuleInfo, ProcessGone, ReadError

PROCESS_VM_READ = 0x0010
PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
STILL_ACTIVE = 259
TH32CS_SNAPPROCESS = 0x00000002
TH32CS_SNAPMODULE = 0x00000008
TH32CS_SNAPMODULE32 = 0x00000010
ERROR_BAD_LENGTH = 24
ERROR_PARTIAL_COPY = 299
ERROR_NO_MORE_FILES = 18
MAX_PATH = 260
MAX_MODULE_NAME32 = 255

_INVALID_HANDLE = ctypes.c_void_p(-1).value


class MODULEENTRY32W(ctypes.Structure):
    _fields_ = [
        ("dwSize", wintypes.DWORD),
        ("th32ModuleID", wintypes.DWORD),
        ("th32ProcessID", wintypes.DWORD),
        ("GlblcntUsage", wintypes.DWORD),
        ("ProccntUsage", wintypes.DWORD),
        ("modBaseAddr", ctypes.c_void_p),
        ("modBaseSize", wintypes.DWORD),
        ("hModule", ctypes.c_void_p),
        ("szModule", ctypes.c_wchar * (MAX_MODULE_NAME32 + 1)),
        ("szExePath", ctypes.c_wchar * MAX_PATH),
    ]


class PROCESSENTRY32W(ctypes.Structure):
    _fields_ = [
        ("dwSize", wintypes.DWORD),
        ("cntUsage", wintypes.DWORD),
        ("th32ProcessID", wintypes.DWORD),
        ("th32DefaultHeapID", ctypes.c_size_t),
        ("th32ModuleID", wintypes.DWORD),
        ("cntThreads", wintypes.DWORD),
        ("th32ParentProcessID", wintypes.DWORD),
        ("pcPriClassBase", wintypes.LONG),
        ("dwFlags", wintypes.DWORD),
        ("szExeFile", ctypes.c_wchar * MAX_PATH),
    ]


def _kernel32():
    if sys.platform != "win32":
        raise AttachError("this tool reads Windows processes; it needs Windows")
    k = ctypes.WinDLL("kernel32", use_last_error=True)
    k.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    k.OpenProcess.restype = wintypes.HANDLE
    k.CloseHandle.argtypes = [wintypes.HANDLE]
    k.CloseHandle.restype = wintypes.BOOL
    k.ReadProcessMemory.argtypes = [
        wintypes.HANDLE,
        ctypes.c_void_p,
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    k.ReadProcessMemory.restype = wintypes.BOOL
    k.IsWow64Process.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.BOOL)]
    k.IsWow64Process.restype = wintypes.BOOL
    k.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    k.GetExitCodeProcess.restype = wintypes.BOOL
    k.QueryFullProcessImageNameW.argtypes = [
        wintypes.HANDLE,
        wintypes.DWORD,
        wintypes.LPWSTR,
        ctypes.POINTER(wintypes.DWORD),
    ]
    k.QueryFullProcessImageNameW.restype = wintypes.BOOL
    k.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
    k.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
    k.Module32FirstW.argtypes = [wintypes.HANDLE, ctypes.POINTER(MODULEENTRY32W)]
    k.Module32FirstW.restype = wintypes.BOOL
    k.Module32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(MODULEENTRY32W)]
    k.Module32NextW.restype = wintypes.BOOL
    k.Process32FirstW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    k.Process32FirstW.restype = wintypes.BOOL
    k.Process32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    k.Process32NextW.restype = wintypes.BOOL
    return k


def find_processes(name: str) -> List[int]:
    """Process ids whose executable file name equals ``name`` (case-insensitive)."""
    k = _kernel32()
    snap = k.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if snap is None or snap == _INVALID_HANDLE:
        raise AttachError("cannot list processes (error %d)" % ctypes.get_last_error())
    found: List[int] = []
    try:
        entry = PROCESSENTRY32W()
        entry.dwSize = ctypes.sizeof(PROCESSENTRY32W)
        ok = k.Process32FirstW(snap, ctypes.byref(entry))
        while ok:
            if entry.szExeFile.lower() == name.lower():
                found.append(int(entry.th32ProcessID))
            ok = k.Process32NextW(snap, ctypes.byref(entry))
    finally:
        k.CloseHandle(snap)
    return found


def wait_for_process(name: str, timeout_s: Optional[float], poll_s: float = 0.25) -> int:
    """Block until exactly one process called ``name`` runs; return its id."""
    deadline = None if timeout_s is None else time.monotonic() + timeout_s
    while True:
        pids = find_processes(name)
        if len(pids) == 1:
            return pids[0]
        if len(pids) > 1:
            raise AttachError(
                "%d processes are called %s (ids %s); pass --pid to choose one"
                % (len(pids), name, ", ".join(str(p) for p in pids))
            )
        if deadline is not None and time.monotonic() >= deadline:
            raise AttachError("no process called %s appeared in %.0f s" % (name, timeout_s))
        time.sleep(poll_s)


class WindowsProcess(Backend):
    """An open handle on one process, with read-only memory access."""

    def __init__(self, pid: int):
        self._k = _kernel32()
        self._pid = int(pid)
        handle = self._k.OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, False, self._pid)
        if not handle:
            error = ctypes.get_last_error()
            raise AttachError("cannot open process %d for reading (Windows error %d)" % (pid, error))
        self._handle = handle
        wow64 = wintypes.BOOL(0)
        if not self._k.IsWow64Process(handle, ctypes.byref(wow64)):
            self.close()
            raise AttachError("IsWow64Process failed (error %d)" % ctypes.get_last_error())
        # A WOW64 process is a 32-bit process on 64-bit Windows; otherwise it matches the OS.
        self.pointer_size = 4 if wow64.value else struct.calcsize("P")
        self.is_wow64 = bool(wow64.value)

    # -- Backend --------------------------------------------------------------------------

    def pid(self) -> int:
        return self._pid

    def read(self, address: int, size: int) -> bytes:
        if self._handle is None:
            raise ProcessGone("handle closed")
        if size < 0 or address < 0 or address + size > (1 << 64):
            raise ReadError("address out of range")
        buffer = (ctypes.c_char * size)()
        done = ctypes.c_size_t(0)
        ok = self._k.ReadProcessMemory(
            self._handle, ctypes.c_void_p(address), buffer, size, ctypes.byref(done)
        )
        if not ok or done.value != size:
            error = ctypes.get_last_error()
            if not self.is_alive():
                raise ProcessGone("the process exited")
            raise ReadError("cannot read %d bytes (Windows error %d)" % (size, error))
        return buffer.raw

    def list_modules(self) -> List[ModuleInfo]:
        for _attempt in range(10):
            snap = self._k.CreateToolhelp32Snapshot(
                TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, self._pid
            )
            if snap is None or snap == _INVALID_HANDLE:
                error = ctypes.get_last_error()
                if error == ERROR_BAD_LENGTH:  # the module list changed while it was copied
                    time.sleep(0.01)
                    continue
                if error == ERROR_PARTIAL_COPY and self.is_alive():
                    return []  # the process is still starting: no module list yet
                if not self.is_alive():
                    raise ProcessGone("the process exited")
                raise AttachError("cannot list the modules (Windows error %d)" % error)
            try:
                modules: List[ModuleInfo] = []
                entry = MODULEENTRY32W()
                entry.dwSize = ctypes.sizeof(MODULEENTRY32W)
                ok = self._k.Module32FirstW(snap, ctypes.byref(entry))
                while ok:
                    modules.append(
                        ModuleInfo(entry.szModule, int(entry.modBaseAddr or 0), int(entry.modBaseSize))
                    )
                    ok = self._k.Module32NextW(snap, ctypes.byref(entry))
                return modules
            finally:
                self._k.CloseHandle(snap)
        raise AttachError("the module list kept changing; try again")

    def is_alive(self) -> bool:
        if self._handle is None:
            return False
        code = wintypes.DWORD(0)
        if not self._k.GetExitCodeProcess(self._handle, ctypes.byref(code)):
            return False
        return code.value == STILL_ACTIVE

    def image_path(self) -> Optional[str]:
        if self._handle is None:
            return None
        size = wintypes.DWORD(32768)
        buffer = ctypes.create_unicode_buffer(size.value)
        if not self._k.QueryFullProcessImageNameW(self._handle, 0, buffer, ctypes.byref(size)):
            return None
        return os.path.normpath(buffer.value)

    def close(self) -> None:
        if self._handle is not None:
            self._k.CloseHandle(self._handle)
            self._handle = None
