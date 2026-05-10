# Contributing to traz

First off, thank you for considering contributing to `traz`! It's people like you that make it a great tool for the community.

## Code of Conduct

By participating in this project, you agree to abide by the terms of the [MIT License](./LICENSE). We aim to maintain a welcoming and inclusive environment.

## How Can I Contribute?

### Reporting Bugs
*   Check the [Issues](https://github.com/mithilgirish/traz/issues) to see if the bug has already been reported.
*   If not, open a new issue. Include a clear title, a description of the problem, and steps to reproduce the issue.

### Suggesting Enhancements
*   Open a new issue with the tag "enhancement".
*   Describe the feature you'd like to see and why it would be useful.

### Pull Requests
1.  **Fork the repository** and create your branch from `main`.
2.  If you've added code that should be tested, **add tests**.
3.  Ensure the test suite passes (`cargo test`).
4.  Run `cargo fmt` to ensure consistent formatting.
5.  Run `cargo clippy` to check for idiomatic Rust improvements.
6.  Open a Pull Request with a clear description of your changes.

## Development Setup

To work on `traz`, you'll need the Rust toolchain installed.

1.  Clone your fork:
    ```bash
    git clone https://github.com/mithilgirish/traz.git
    cd traz
    ```
2.  Build the project:
    ```bash
    cargo build
    ```
3.  Run the tests:
    ```bash
    cargo test
    ```

## Style Guidelines

*   Follow standard Rust naming conventions (snake_case for functions/variables, PascalCase for types).
*   Keep functions small and focused.
*   Write comments for complex logic.
*   Update the `README.md` if you change any CLI commands or features.

## Questions?

Feel free to open an issue with the "question" tag!
