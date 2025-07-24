# Repository Instructions for Codex Agents

## Development Workflow

1. Always allow your environment setup script to run till the end. It installs dependencies you may not be able later to fetch.
2. It is absolutely forbidden to touch Cargo.toml and Cargo.lock. If you need to make changes in Cargo.toml or Cargo.lock, output instead the lines you would like to add or remove, explain why, and the human will do this for you if it makes sense.
3. Follow Rust format conventions but do not format any code you do not change.
4. Ensure the code builds and tests pass using `cargo check` and `cargo test`. Always run the complete test suite before commiting, not just specifically for the features you worked on.
5. It is totally unacceptable to commit code that does not compile.
6. It is totally unacceptable to commit code that breaks irrelevant tests, but new tests on incomplete features you are working on may fail. 
7. When you write test for serialization, set field values and verify YAML output.
8. When you write tests for deserialization, set YAML and verify the output. Do now just write "round trip" not caring what was in between.
9. When you write tests, use Serde with a sample structure, not with Value. If you use structure in just one test, put it into the body of the test.

## Pull Request

- Summarize the changes and mention the results of running `cargo check` and `cargo test`.
- Include any necessary notes about limitations or skipped steps.
