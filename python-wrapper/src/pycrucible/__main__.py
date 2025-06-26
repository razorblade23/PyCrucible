import os
import subprocess
import sys
from pathlib import Path

def main():
    bin_path = Path(__file__).parent / "pycrucible-bin"
    result = subprocess.run([str(bin_path)] + sys.argv[1:], check=False)
    sys.exit(result.returncode)
