"""Entry point: python -I tools/oracle/oracle.py --help

`python -I` leaves the script folder off sys.path, so the package next to this file is added
explicitly (it is this tool's own code, not a lookup of whatever is in the current directory).
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from ss2_oracle.cli import main  # noqa: E402

if __name__ == "__main__":
    sys.exit(main())
