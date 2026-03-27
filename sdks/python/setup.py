from setuptools import setup, find_packages

setup(
    name="hsip-sdk",
    version="0.1.0",
    description="HSIP Python SDK - Cryptographic consent and message verification",
    packages=find_packages(),
    python_requires=">=3.8",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: Other/Proprietary License",
        "Topic :: Security :: Cryptography",
    ],
)
