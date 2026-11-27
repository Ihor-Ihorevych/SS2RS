"""Test doubles for the oracle harness: a fake process and a fake clock. Not used outside tests."""

import struct
from typing import Callable, Dict, List, Optional

from ss2_oracle.backend import Backend, ModuleInfo, ProcessGone, ReadError


class FakeProcess(Backend):
    """A process whose memory is a dict of regions: base address -> bytes."""

    def __init__(self, pointer_size: int = 4):
        self.pointer_size = pointer_size
        self.regions: Dict[int, bytearray] = {}
        self.modules: List[ModuleInfo] = []
        self.alive = True
        self.reads: List[tuple] = []
        self.on_read: Optional[Callable[[], None]] = None
        self.exe = "C:\\fake\\Bin\\Sam2.exe"
        self.module_list_calls = 0

    # -- building the fake ----------------------------------------------------------------

    def add_module(self, name: str, base: int, size: int = 0x10000) -> None:
        self.modules.append(ModuleInfo(name, base, size))
        self.regions.setdefault(base, bytearray(size))

    def poke(self, address: int, data: bytes) -> None:
        for base, region in self.regions.items():
            if base <= address and address + len(data) <= base + len(region):
                region[address - base : address - base + len(data)] = data
                return
        raise AssertionError("poke outside every region: 0x%X" % address)

    def poke_ptr(self, address: int, value: int) -> None:
        self.poke(address, value.to_bytes(self.pointer_size, "little"))

    def poke_f32(self, address: int, value: float) -> None:
        self.poke(address, struct.pack("<f", value))

    def add_region(self, base: int, size: int) -> None:
        self.regions[base] = bytearray(size)

    # -- Backend --------------------------------------------------------------------------

    def read(self, address: int, size: int) -> bytes:
        if not self.alive:
            raise ProcessGone("exited")
        self.reads.append((address, size))
        if self.on_read:
            self.on_read()
        for base, region in self.regions.items():
            if base <= address and address + size <= base + len(region):
                return bytes(region[address - base : address - base + size])
        raise ReadError("unmapped 0x%X" % address)

    def list_modules(self) -> List[ModuleInfo]:
        if not self.alive:
            raise ProcessGone("exited")
        self.module_list_calls += 1
        return list(self.modules)

    def is_alive(self) -> bool:
        return self.alive

    def image_path(self) -> Optional[str]:
        return self.exe

    def pid(self) -> int:
        return 4242


class FakeClock:
    """A clock the test moves by hand; ``sleep`` advances it."""

    def __init__(self, start: float = 1000.0):
        self.now = start
        self.sleeps: List[float] = []

    def __call__(self) -> float:
        return self.now

    def sleep(self, seconds: float) -> None:
        self.sleeps.append(seconds)
        self.now += max(0.0, seconds)

    def advance(self, seconds: float) -> None:
        self.now += seconds
