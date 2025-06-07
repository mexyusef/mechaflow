# Contributing to MechaFlow

Thank you for your interest in contributing to MechaFlow! This document provides guidelines and instructions for contributing to this project.

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/mexyusef/mechaflow.git`
3. Create a branch for your changes: `git checkout -b feature/your-feature-name`

## Development Environment

### Rust Components

1. Install Rust using [rustup](https://rustup.rs/)
2. Navigate to the Rust component directory
3. Run `cargo build` to compile
4. Run `cargo test` to execute tests

### Python Components

1. Create a virtual environment: `python -m venv venv`
2. Activate the environment:
   - Windows: `venv\Scripts\activate`
   - Unix/MacOS: `source venv/bin/activate`
3. Install dependencies: `pip install -e ".[dev]"`
4. Run tests: `pytest`

## Pull Request Process

1. Update the README.md or documentation with details of changes if appropriate
2. Update the CHANGELOG.md with details of changes
3. The PR should work for all supported platforms
4. Ensure all tests pass
5. Request a review from a maintainer

## Coding Standards

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format your code
- Run `cargo clippy` and address any warnings

### Python

- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/) style guide
- Use type hints where appropriate
- Document public functions, classes, and methods

## Testing

- Write tests for new features or bug fixes
- Maintain or improve code coverage
- Tests should be fast and not depend on external services

## Documentation

- Update documentation for any changed functionality
- Add docstrings to public APIs
- Consider adding examples for new features

## Licensing

By contributing to MechaFlow, you agree that your contributions will be licensed under the project's MIT License.

## Questions?

Don't hesitate to open an issue with questions or suggestions!
