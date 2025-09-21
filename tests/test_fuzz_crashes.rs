use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn collect_test_inputs(base: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut inputs = Vec::new();
    if !base.exists() {
        return Ok(inputs);
    }
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
           inputs.push(path);
        }
    }
    Ok(inputs)
}

// Parse all documents from bytes using serde_yaml. If the input is not valid UTF-8
// or YAML, this returns an error.
fn parse_all_with_serde_yaml_from_bytes(input: &[u8]) -> anyhow::Result<Vec<serde_yaml::Value>> {
    let mut docs = Vec::new();
    let des = serde_yaml::Deserializer::from_slice(input);
    for doc in des {
        let v = serde_yaml::Value::deserialize(doc)?;
        docs.push(v);
    }
    Ok(docs)
}

fn parse_all_with_bw_from_bytes(input: &[u8]) -> serde_yaml_bw::Result<Vec<serde_yaml_bw::Value>> {
    serde_yaml_bw::from_slice_multi::<serde_yaml_bw::Value>(input)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Ok,
    Error,
    Crash,
}

fn classify_serde(input: &[u8]) -> (Status, Option<Vec<serde_yaml::Value>>) {
    let res = catch_unwind(AssertUnwindSafe(|| parse_all_with_serde_yaml_from_bytes(input)));
    match res {
        Err(_) => (Status::Crash, None),
        Ok(Err(_)) => (Status::Error, None),
        Ok(Ok(v)) if v.is_empty() => (Status::Error, None),
        Ok(Ok(v)) => (Status::Ok, Some(v)),
    }
}

fn classify_bw(input: &[u8]) -> (Status, Option<Vec<serde_yaml_bw::Value>>) {
    let res = catch_unwind(AssertUnwindSafe(|| parse_all_with_bw_from_bytes(input)));
    match res {
        Err(_) => (Status::Crash, None),
        Ok(Err(_)) => (Status::Error, None),
        Ok(Ok(v)) if v.is_empty() => (Status::Error, None),
        Ok(Ok(v)) => (Status::Ok, Some(v)),
    }
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Ok => "ok",
        Status::Error => "error",
        Status::Crash => "crash",
    }
}

#[test]
fn yaml_test_suite_differential() -> Result<()> {
    let base = Path::new("tests/fuzz_crashes");
    if !base.exists() {
        eprintln!("tests/fuzz_crashes directory not found; skipping differential test");
        return Ok(());
    }

    let inputs = collect_test_inputs(base)?;
    if inputs.is_empty() {
        eprintln!("No files found. Skipping differential test");
        return Ok(());
    }

    let mut tested = 0usize;
    let mut skipped = 0usize;

    for file in inputs {
        let yaml_bytes = fs::read(&file)
            .with_context(|| format!("reading {}", file.display()))?;

        // First, classify serde_yaml outcome and capture docs if successful.
        let (serde_status, ser_docs_opt) = classify_serde(&yaml_bytes);
        if serde_status != Status::Ok {
            // Also classify our parser for summary purposes, but do not assert.
            let (bw_status, _bw_docs_opt) = classify_bw(&yaml_bytes);
            eprintln!(
                "summary: {} | ours: {} | serde_yaml: {}",
                file.display(), status_str(bw_status), status_str(serde_status)
            );
            skipped += 1;
            continue;
        }
        let ser_docs = ser_docs_opt.expect("status Ok must include docs");

        // Our parser must be able to parse if serde_yaml did. Catch crashes to report nicely.
        let bw_attempt = catch_unwind(AssertUnwindSafe(|| parse_all_with_bw_from_bytes(&yaml_bytes)));
        let bw_docs = match bw_attempt {
            Err(_) => {
                eprintln!(
                    "summary: {} | ours: crash | serde_yaml: ok",
                    file.display()
                );
                let yaml_preview = String::from_utf8_lossy(&yaml_bytes);
                panic!(
                    "Our parser crashed on a case that serde_yaml accepted.\nFile: {}\nInput (lossy):\n{}",
                    file.display(), yaml_preview
                );
            }
            Ok(Ok(v)) => {
                eprintln!(
                    "summary: {} | ours: ok | serde_yaml: ok",
                    file.display()
                );
                v
            }
            Ok(Err(err)) => {
                eprintln!(
                    "summary: {} | ours: error | serde_yaml: ok",
                    file.display()
                );
                let yaml_preview = String::from_utf8_lossy(&yaml_bytes);
                panic!(
                    "Our parser failed to parse a case that serde_yaml accepted.\nFile: {}\nError: {err}\nInput (lossy):\n{}",
                    file.display(), yaml_preview
                );
            }
        };

        // Serialize our docs back to YAML using our serializer and compare by
        // re-parsing with serde_yaml into Values, then equality check.
        let bw_yaml = if bw_docs.len() == 1 {
            serde_yaml_bw::to_string(&bw_docs[0])?
        } else {
            serde_yaml_bw::to_string_multi(&bw_docs)?
        };

        let reparsed_by_serde = parse_all_with_serde_yaml_from_bytes(bw_yaml.as_bytes())?;
        assert_eq!(
            reparsed_by_serde, ser_docs,
            "Roundtrip via our serializer/Value changed semantics compared to serde_yaml.\nFile: {}\nOur emitted YAML:\n{}",
            file.display(), bw_yaml
        );

        // Additionally, serialize the serde_yaml Values using our serializer
        // and ensure our parser reads them back to an equivalent structure
        // (as judged again by serde_yaml).
        let ser_yaml_via_bw = if ser_docs.len() == 1 {
            serde_yaml_bw::to_string(&ser_docs[0])?
        } else {
            serde_yaml_bw::to_string_multi(&ser_docs)?
        };

        let reparsed_bw = parse_all_with_bw_from_bytes(ser_yaml_via_bw.as_bytes())?;
        let reparsed_bw_via_serde = parse_all_with_serde_yaml_from_bytes(
            serde_yaml_bw::to_string_multi(&reparsed_bw)?.as_bytes(),
        )?;
        assert_eq!(
            reparsed_bw_via_serde, ser_docs,
            "Serializing serde_yaml Values with our serializer, then parsing with our parser, should be semantics-preserving.\nFile: {}\nserde_yaml -> (our serializer) YAML:\n{}",
            file.display(), ser_yaml_via_bw
        );

        tested += 1;
    }

    eprintln!("yaml-test-suite differential: tested {} cases, skipped {}", tested, skipped);
    Ok(())
}
