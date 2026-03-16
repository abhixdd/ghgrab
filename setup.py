from setuptools import setup, find_packages
from setuptools.command.build_py import build_py
from wheel.bdist_wheel import bdist_wheel as _bdist_wheel
import os
import sys
import urllib.request
import platform
from pathlib import Path
import hashlib
import json

VERSION = "1.0.1"

def get_platform_info():
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "windows":
        return "win32", "ghgrab-win32.exe", "ghgrab.exe"
    elif system == "darwin":
        if machine in ("arm64", "aarch64"):
            return "darwin-arm64", "ghgrab-darwin-arm64", "ghgrab"
        return "darwin", "ghgrab-darwin", "ghgrab"
    elif system == "linux":
        if machine in ("arm64", "aarch64"):
            return "linux-arm64", "ghgrab-linux-arm64", "ghgrab"
        return "linux", "ghgrab-linux", "ghgrab"
    return None, None, None

def download_binary():
    platform_name, remote_name, local_name = get_platform_info()
    if not platform_name:
        return

    bin_dir = Path(__file__).parent / "ghgrab"
    bin_dir.mkdir(parents=True, exist_ok=True)
    bin_path = bin_dir / local_name

    if bin_path.exists() and bin_path.stat().st_size > 100_000:
        print(f"Using existing binary at {bin_path}")
        return

    url = f"https://github.com/abhixdd/ghgrab/releases/download/v{VERSION}/{remote_name}"
    print(f"Downloading ghgrab v{VERSION} binary for {platform_name} from {url}...")
    try:
        urllib.request.urlretrieve(url, bin_path)
        size = bin_path.stat().st_size
        if size < 100_000:
            bin_path.unlink(missing_ok=True)
            print(f"Downloaded file too small ({size} bytes). Skipping.")
            return

        checksum_file = Path(__file__).parent / "checksums.json"
        if checksum_file.exists():
            with open(checksum_file, "r") as f:
                checksums = json.load(f)
            expected_hash = checksums.get(remote_name)
            if expected_hash:
                sha256_hash = hashlib.sha256()
                with open(bin_path, "rb") as f:
                    for byte_block in iter(lambda: f.read(4096), b""):
                        sha256_hash.update(byte_block)
                actual_hash = sha256_hash.hexdigest()
                if actual_hash != expected_hash:
                    bin_path.unlink(missing_ok=True)
                    raise ValueError(f"Checksum mismatch for {remote_name}. Expected {expected_hash}, got {actual_hash}.")
                print(f"✓ Checksum verified for {remote_name}")

        if platform.system().lower() != "windows":
            bin_path.chmod(0o755)
        print(f"✓ Binary downloaded to {bin_path}")
    except Exception as e:
        print(f"Warning: Could not download binary: {e}")

class BuildPy(build_py):
    def run(self):
        download_binary()
        super().run()

class bdist_wheel(_bdist_wheel):
    def finalize_options(self):
        super().finalize_options()

        self.root_is_pure = False

    def get_tag(self):
        python, abi, plat = super().get_tag()
        

        if plat.startswith("linux_x86_64"):
            plat = "manylinux2014_x86_64"
        elif plat.startswith("linux_aarch64"):
            plat = "manylinux2014_aarch64"
            
        return "py3", "none", plat

setup(
    name="ghgrab",
    version=VERSION,
    packages=find_packages(),
    package_data={"ghgrab": ["ghgrab", "ghgrab.exe"]},
    include_package_data=True,
    cmdclass={
        "build_py": BuildPy,
        "bdist_wheel": bdist_wheel,
    },
    zip_safe=False,
)
