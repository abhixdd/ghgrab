from setuptools import setup, find_packages
from setuptools.command.install import install
import os
import sys
import urllib.request
import platform

class PostInstallCommand(install):
    def run(self):
        install.run(self)
        self.download_binary()
    
    def download_binary(self):
        version = "0.1.0"
        system = platform.system().lower()
        
        if system == "windows":
            platform_name = "win32"
            binary_name = "ghg  rab.exe"
        elif system == "darwin":
            arch = platform.machine()
            platform_name = "darwin-arm64" if arch == "arm64" else "darwin"
            binary_name = "ghgrab"
        elif system == "linux":
            platform_name = "linux"
            binary_name = "ghgrab"
        else:
            print(f"Unsupported platform: {system}")
            return
        
        url = f"https://github.com/abhixdd/ghgrab/releases/download/v{version}/ghgrab-{platform_name}"
        bin_dir = os.path.join(os.path.dirname(__file__), "ghgrab")
        bin_path = os.path.join(bin_dir, binary_name)
        
        try:
            print(f"Downloading ghgrab binary for {platform_name}...")
            urllib.request.urlretrieve(url, bin_path)
            os.chmod(bin_path, 0o755)
            print("Binary downloaded successfully!")
        except Exception as e:
            print(f"Failed to download binary: {e}")

setup(
    name="ghgrab",
    version="0.1.0",
    packages=find_packages(),
    cmdclass={
        'install': PostInstallCommand,
    },
    zip_safe=False,
)
