use std::fs;
use std::path::{Path, PathBuf};

fn collect_yaml_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
                files.push(path);
            }
        }
    }
    Ok(())
}

#[test]
fn yaml_test_suite_no_crash() -> anyhow::Result<()> {
    let base = Path::new("tests/yaml-test-suite/src");
    if !base.exists() {
        eprintln!("yaml-test-suite submodule not found; skipping");
        return Ok(());
    }

    let mut files = Vec::new();
    collect_yaml_files(base, &mut files)?;

    if files.is_empty() {
        eprintln!("No YAML files found in yaml-test-suite; skipping");
        return Ok(());
    }

    for file in files {
        let text = fs::read_to_string(&file)?;
        let _ = serde_yaml_bw::from_str::<serde_yaml_bw::Value>(&text);
    }

    Ok(())
}
