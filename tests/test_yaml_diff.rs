use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
fn collect_test_inputs(base: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut inputs = Vec::new();
    if !base.exists() {
        return Ok(inputs);
    }
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" {
                    inputs.push(path);
                }
            }
        }
    }
    Ok(inputs)
}


fn parse_all_with_serde_yaml(input: &str) -> anyhow::Result<Vec<serde_yaml::Value>> {
    let mut docs = Vec::new();
    let des = serde_yaml::Deserializer::from_str(input);
    for doc in des {
        let v = serde_yaml::Value::deserialize(doc)?;
        docs.push(v);
    }
    Ok(docs)
}

fn parse_all_with_bw(input: &str) -> serde_yaml_bw::Result<Vec<serde_yaml_bw::Value>> {
    serde_yaml_bw::from_str_multi::<serde_yaml_bw::Value>(input)
}

fn parse_all_with_serde_yaml_from_str(input: &str) -> anyhow::Result<Vec<serde_yaml::Value>> {
    // Some inputs may be single-document. This function always returns a vector.
    // Using Deserializer already handles multi-doc, so reuse it.
    parse_all_with_serde_yaml(input)
}

fn parse_all_with_bw_from_str(input: &str) -> serde_yaml_bw::Result<Vec<serde_yaml_bw::Value>> {
    parse_all_with_bw(input)
}

#[test]
fn yaml_test_suite_differential() -> Result<()> {
    let base = Path::new("tests/yaml-test-suite/src");
    if !base.exists() {
        eprintln!("yaml-test-suite 'src' directory not found; skipping differential test");
        return Ok(());
    }

    let inputs = collect_test_inputs(base)?;
    if inputs.is_empty() {
        eprintln!("No in.yaml files found in yaml-test-suite; skipping differential test");
        return Ok(());
    }

    let mut tested = 0usize;
    let mut skipped = 0usize;

    for file in inputs {
        let yaml = fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;

        // Use serde_yaml as the reference. If it can't parse, skip this case.
        let ser_docs = match parse_all_with_serde_yaml_from_str(&yaml) {
            Ok(v) if !v.is_empty() => v,
            Ok(_) => { skipped += 1; continue; },
            Err(_e) => { skipped += 1; continue; },
        };

        // Our parser must be able to parse if serde_yaml did.
        let bw_docs = match parse_all_with_bw_from_str(&yaml) {
            Ok(v) => v,
            Err(err) => {
                panic!(
                    "Our parser failed to parse a case that serde_yaml accepted.\nFile: {}\nError: {err}\nInput:\n{}",
                    file.display(), yaml
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

        let reparsed_by_serde = parse_all_with_serde_yaml(&bw_yaml)?;
        assert_eq!(
            reparsed_by_serde, ser_docs,
            "Roundtrip via our serializer/Value changed semantics compared to serde_yaml.\nFile: {}\nInput:\n{}\nOur emitted YAML:\n{}",
            file.display(), yaml, bw_yaml
        );

        // Additionally, serialize the serde_yaml Values using our serializer
        // and ensure our parser reads them back to an equivalent structure
        // (as judged again by serde_yaml).
        let ser_yaml_via_bw = if ser_docs.len() == 1 {
            serde_yaml_bw::to_string(&ser_docs[0])?
        } else {
            serde_yaml_bw::to_string_multi(&ser_docs)?
        };

        let reparsed_bw = parse_all_with_bw(&ser_yaml_via_bw)?;
        let reparsed_bw_via_serde = parse_all_with_serde_yaml(&serde_yaml_bw::to_string_multi(&reparsed_bw)?)?;
        assert_eq!(
            reparsed_bw_via_serde, ser_docs,
            "Serializing serde_yaml Values with our serializer, then parsing with our parser, should be semantics-preserving.\nFile: {}\nInput:\n{}\nserde_yaml -> (our serializer) YAML:\n{}",
            file.display(), yaml, ser_yaml_via_bw
        );

        tested += 1;
    }

    eprintln!("yaml-test-suite differential: tested {} cases, skipped {}", tested, skipped);
    Ok(())
}
