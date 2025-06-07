#!/usr/bin/env python3
from setuptools import setup, find_packages
from setuptools_rust import Binding, RustExtension

setup(
    name="pymechaflow",
    version="0.0.1",
    author="Yusef Ulum",
    author_email="yusef314159@gmail.com",
    description="Python bindings for the MechaFlow automation framework",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="https://github.com/mexyusef/mechaflow",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Rust",
        "Topic :: Software Development :: Libraries",
        "Topic :: Home Automation",
    ],
    python_requires=">=3.7",
    rust_extensions=[
        RustExtension(
            "pymechaflow.pymechaflow",
            binding=Binding.PyO3,
            features=["extension-module"],
        )
    ],
    include_package_data=True,
    zip_safe=False,
)
