#!/usr/bin/env python3
import os
import sys
import subprocess
from pathlib import Path

def main():
    # Get the directory where the binary is installed
    bin_dir = Path(__file__).parent
    binary_name = "ghgrab.exe" if sys.platform == "win32" else "ghgrab"
    binary_path = bin_dir / binary_name
    
    # Execute the Rust binary
    try:
        result = subprocess.run([str(binary_path)] + sys.argv[1:])
        sys.exit(result.returncode)
    except FileNotFoundError:
        print(f"Error: Binary not found at {binary_path}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
