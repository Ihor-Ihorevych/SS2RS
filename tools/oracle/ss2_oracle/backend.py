"""The interface the harness reads another process through, and its errors.

The real implementation is ``winproc.WindowsProcess``; the tests use a fake that serves bytes
from a dict. There is deliberately **no write method** on this interface.
"""

from dataclasses import dataclass
from typing import List, Optional


class BackendError(Exception):
    """Base class of the errors a backend raises."""


class ReadError(BackendError):
    """The bytes at that address cannot be read (unmapped page, partial copy)."""


class ProcessGone(BackendError):
    """The target process has exited."""


class AttachError(BackendError):
    """The process could not be found or opened."""


@dataclass(frozen=True)
class ModuleInfo:
    name: str
    base: int
    size: int


class Backend:
    """Read-only view of one running process."""

    #: pointer width of the target in bytes (4 for the 32-bit game)
    pointer_size: int = 4

    def read(self, address: int, size: int) -> bytes:
        """Return exactly ``size`` bytes or raise ReadError / ProcessGone."""
        raise NotImplementedError

    def list_modules(self) -> List["ModuleInfo"]:
        raise NotImplementedError

    def is_alive(self) -> bool:
        raise NotImplementedError

    def image_path(self) -> Optional[str]:
        """Full path of the process's executable, when known."""
        return None

    def pid(self) -> Optional[int]:
        return None

    def close(self) -> None:
        pass

    def module_base(self, name: str) -> Optional[int]:
        """Base address of the module called ``name`` (case-insensitive), or None."""
        wanted = name.lower()
        for module in self.list_modules():
            if module.name.lower() == wanted:
                return module.base
        return None
