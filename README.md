[![GitHub](https://img.shields.io/badge/GitHub-777777)](https://github.com/bourumir-wyngs/serde-yaml-bw)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)
[![crates.io](https://img.shields.io/crates/v/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/l/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/d/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![docs.rs](https://docs.rs/serde_yaml_bw/badge.svg)](https://docs.rs/serde_yaml_bw)

This package is a fork of `serde-yaml`, more intended as a temporary solution until a reasonable
replacement emerges. It significantly reduces the number of `panic!()` and `.unwrap()` constructs, opting instead to
return proper error messages rather than crashing outright. This makes the library suitable for parsing user-supplied
YAML content.

The initiative began as an effort to continue maintaining the
widely-used [serde_yaml](https://github.com/dtolnay/serde-yaml) library, which has since been archived and marked as
deprecated on GitHub. Following the fork, minor updates were applied, including advancing some package version numbers,
incorporating additional tests from
Fishrock123's [abandoned pull request](https://github.com/dtolnay/serde-yaml/pull/376), and improving error messages to
clearly indicate unresolved YAML anchors.

We have upgraded it to the Rust 2024 edition.

The package is somewhat maintained as it is used in our own projects.







