# Contributing

Contributions are welcome, whether they are bug fixes, improvements, documentation updates, or new features (if aligned with the project scope).

## How to Contribute

0. **Open a feature request first**
    - Before implementing a new feature, open an issue to discuss it.
    - Provide use cases, and link to relevant specification documents where possible.
    - Submitting a surprise pull request without prior discussion often leads to wasted effort and frustration on all sides.

1. **Fork and branch**
    - Fork the repository and create a feature branch from `main`.

2. **Code style**
    - Follow standard Rust coding style and conventions.
    - Use `rustfmt` and `clippy` to keep the code consistent and clean.
    - It should be no warnings in new code.
    - Keep changes minimal, focused, and easy to review.

3. **Tests**
    - Add or update tests for any new code.
    - Ensure that all tests pass locally before submitting a pull request.

4. **Commits**
    - Write clear, descriptive commit messages.
    - Keep commits logically organized (squash when appropriate).

5. **Pull requests**
    - Submit your pull request against `main`.
    - Include a concise description of the change and its motivation.
    - Be open to feedback and willing to revise your code as needed.


## AI Usage

AI-assisted (not vibe-coded!) contributions are acceptable and will be reviewed by the **same standards** as any other code. AI itself is less a problem, what matters is "low effort content".

A submission may be rejected if it includes any of the following:

- **Unnecessary rewrites**  
  Drive-by refactors or widespread cosmetic edits unrelated to the stated goal. Keep diffs minimal and focused to respect reviewers’ time.

- **Overcomplicated solutions**  
  Prefer clear, idiomatic Rust over convoluted designs. Solve the problem as simply as possible.

- **Features outside YAML 1.1/1.2**  
  Do not propose features that are not part of the YAML 1.1 or 1.2 specifications. Keeping scope lean helps the project evolve without feature bloat.

- **Excessive PR volume**  
  A large number of small PRs from the same author in a short time often indicates insufficient self-review. Consolidate related changes.

- **Missing or weak tests**  
  Include meaningful tests. AI can be good at generating tests, but you must guide it and ensure tests actually verify behavior (and fail when they should).

- **Unvetted or undisclosed dependencies**  
  Do not add dependencies without prior discussion. Verify that a package exists, is maintained, appropriately licensed, and reputable. AI tools sometimes invent non-existent crates.

In short: **you are responsible for the quality of your contribution**. AI is a tool like a text editor. It can also help improve writing if you are a non-native speaker, but avoid producing long, verbose text just because you can—the result is as annoying as when done poorly by a human.


## Reporting Issues

- Use the issue tracker to report bugs or request features.
- Provide clear steps to reproduce bugs where possible (ideally the test case)

## Code of Conduct

Be respectful and constructive. This project values collaboration and clarity. Disrespectful or unprofessional behavior will not be tolerated.
