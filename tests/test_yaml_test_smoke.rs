use std::fs;
use std::path::{Path, PathBuf};

/// Reads a YAML file and processes special unicode indicators into real characters.
/// These conversions are specified in YAML test suite documentation.
///
/// Special unicode indicators handled:
/// - '␣' (U+2423) → Space (' ')
/// - '———»', '——»', '—»', '»' → Tab ('\t')
/// - '↵' (U+21B5) → Newline ('\n')
/// - '∎' (U+220E) → Removes final newline (ensures no trailing newline)
/// - '←' (U+2190) → Carriage return ('\r')
/// - '⇔' (U+21D4) → Byte Order Mark (BOM, U+FEFF)
///
/// Event output indicators (optional, can be uncommented if used):
/// - '<SPC>' → ' '
/// - '<TAB>' → '\t'
///
/// # Arguments
/// * `file` - The path to the input YAML file.
///
/// # Returns
/// A processed YAML content as a String, or an error.
fn read_yaml(file: &PathBuf) -> anyhow::Result<String> {
    let mut content = fs::read_to_string(file)?;

    // Replace special unicode indicators
    content = content
        .replace('␣', " ")
        .replace("———»", "\t")
        .replace("——»", "\t")
        .replace("—»", "\t")
        .replace('»', "\t")
        .replace('↵', "\n")
        .replace('←', "\r")
        .replace('⇔', "\u{FEFF}");

    // Handle '∎' (no final newline character)
    if content.ends_with('∎') {
        content.pop(); // remove '∎'
        content = content.trim_end_matches('\n').to_string(); // ensure no trailing newline
    } else {
        // Otherwise, ensure exactly one trailing newline
        content = content.trim_end_matches('\n').to_string() + "\n";
    }
    Ok(content)
}

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


// Smoke test only so far: we do not check if it is parsing correctly yet but
// absolutely should not crash.
#[test]
fn yaml_test_suite_smoke() -> anyhow::Result<()> {
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

    println!("Testing {} cases", files.len());

    for file in files {
        let yaml = read_yaml(&file)?;
        let _ = serde_yaml_bw::from_str::<serde_yaml_bw::Value>(&yaml);
    }

    Ok(())
}
