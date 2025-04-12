[![GitHub](https://img.shields.io/badge/GitHub-777777)](https://github.com/bourumir-wyngs/serde-yaml-bw)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)
[![crates.io](https://img.shields.io/crates/v/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/l/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/d/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![docs.rs](https://docs.rs/serde_yaml_bw/badge.svg)](https://docs.rs/serde_yaml_bw)

This package is the branch of serde-yaml that is minimally maintained before a reasonable replacement would emerge. 

It started as the initiative to take over and further maintain the highly popular
[serde_yaml](https://github.com/dtolnay/serde-yaml) library after it has been set as read only on GitHub and also
marked as deprecated. We no longer have these plans. Some very minimal work has been done after forking,
like version number of some packages have been advanced a little bit, some more tests have been added from
the [abandoned pull request](https://github.com/dtolnay/serde-yaml/pull/376) by Fishrock123 as in GitHub and
the sanitized text of the unresolved anchor is now included in the error message. That's it.

For 1.0.2 we updated Rust to edition 2024, this required some tweaks in the code. 



