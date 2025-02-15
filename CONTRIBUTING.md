# Contributing to Nephelios 🚀

Thank you for your interest in contributing to **Nephelios**! Your contributions help make this open-source project better for everyone. This guide will walk you through the process of contributing effectively.

## Table of Contents

1. [Code of Conduct](#code-of-conduct-)
2. [How to Contribute](#how-to-contribute-)
    - [Reporting Bugs](#reporting-bugs-)
    - [Suggesting Features](#suggesting-features-)
    - [Submitting a Pull Request](#submitting-a-pull-request-pr-)
3. [Setting Up Your Development Environment](#setting-up-your-development-environment-)
4. [Code Style](#code-style-)
5. [Testing](#testing-)
6. [Getting Help](#getting-help-)

---

## Code of Conduct 📜

Please note that this project follows a **[Code of Conduct](./CODE_OF_CONDUCT.md)**. By participating, you agree to maintain a welcoming and respectful environment for all contributors.

---

## How to Contribute 🛠️

### Reporting Bugs 🐛

If you find a bug, please open an **issue** with the following details:

- A **clear description** of the problem.
- Steps to **reproduce the bug**.
- Expected vs. actual behavior.
- Any relevant **logs** or **error messages**.

### Suggesting Features 💡

Have an idea for improving Nephelios? Great! Before opening a feature request:

- Check if a similar **suggestion** already exists.
- Open an **issue** describing:
    - The **feature** you propose.
    - Why it would be beneficial.
    - Any **alternatives** you've considered.

### Submitting a Pull Request (PR) 📌

To contribute code:

1. **Fork** this repository.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/your-username/nephelios.git
   cd nephelios
   ```
3. **Create a new branch** for your changes:
   ```bash
   git checkout -b feature/my-feature
   ```
4. **Implement your changes** and ensure they follow the [Code Style](#code-style-).
5. **Run tests** before committing (`cargo test`).
6. **Commit your changes**:
   ```bash
   git commit -m "feat: Add my new feature"
   ```
7. **Push your branch**:
   ```bash
   git push origin feature/my-feature
   ```
8. **Open a Pull Request** on GitHub.

Make sure your PR:
- Includes a **clear description** of the changes.
- Links to a relevant **issue** if applicable.
- Passes all **tests**.

---

## Setting Up Your Development Environment ⚙️

### Prerequisites

Ensure you have the following installed:
- [Rust](https://www.rust-lang.org/) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/)
- [Docker](https://www.docker.com/) (optional for testing)

### Build & Run

To build the project:
```bash
cargo build
```

To run the project:
```bash
cargo run
```

### Environment Configuration

Copy the example environment file and adjust the settings as needed:
```bash
cp .env.example .env
```

---

## Code Style 🖋️

We follow **Rust’s standard formatting and linting**. Please ensure your code follows these guidelines before submitting a PR:

- Format your code with:
  ```bash
  cargo fmt
  ```
- Lint your code with:
  ```bash
  cargo clippy
  ```
- Follow best practices from [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

---

## Testing 🧪

Before submitting a PR, run the tests to ensure everything is working:
```bash
cargo test
```

If your changes introduce new functionality, please **write tests** for it.

---

## Getting Help ❓

If you need help:
- Check existing **issues** and **discussions**.
- Open a **new issue** if you can't find an answer.
- Reach out to the **maintainers** or the **community**.

---

Thank you for your contribution! 🎉 Your help makes Nephelios better. 🚀