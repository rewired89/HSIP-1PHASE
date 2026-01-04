"""
HSIP Bank SDK Setup
"""

from setuptools import setup, find_packages
from pathlib import Path

# Read README for long description (with UTF-8 encoding for Windows compatibility)
readme = Path(__file__).parent / "README.md"
long_description = readme.read_text(encoding="utf-8") if readme.exists() else ""

setup(
    name="hsip-bank-sdk",
    version="1.0.0",
    description="HSIP Backend SDK for banks to sign verified messages",
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="Nyx Systems LLC",
    author_email="contact@nyxsystems.com",
    url="https://github.com/nyxsystems/hsip-bank-sdk",
    packages=find_packages(),
    python_requires=">=3.8",
    install_requires=[
        "cryptography>=41.0.0",
    ],
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: Other/Proprietary License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Security :: Cryptography",
        "Topic :: Software Development :: Libraries",
    ],
    keywords="hsip security banking messaging authentication cryptography",
    entry_points={
        "console_scripts": [
            "hsip-keygen=hsip_bank.cli:keygen_cli",
        ],
    },
)
