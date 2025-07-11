# Repository Instructions for Codex Agents

## Development Workflow

1. Always allow your environment setup script to run till the end. It installs dependencies you may not be able later to fetch.
2. It is absolutely forbidden to touch Cargo.toml and Cargo.lock. Ask the human to do this for you.
3. Follow Rust format conventions but do not format any code you do not change.
3. Ensure the code builds and tests pass using `cargo check` and `cargo test`. Always run the complete test suite before commiting, not just specifically for the features you worked on.
5. It is totally unacceptable to commit code that does not compile.
6. It is totally unacceptable to commit code that breaks irrelevant tests, but new tests on incomplete features you are working on may fail. 

## Pull Request

- Summarize the changes and mention the results of running `cargo check` and `cargo test`.
- Include any necessary notes about limitations or skipped steps.
