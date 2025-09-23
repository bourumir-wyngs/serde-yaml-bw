//! Detecting the knowingly invalid YAML that can result in very slow parsing,
//! opening possibility for DOS attacks.
use crate::error::{PathologicalYaml, PathologyReason};

/// Tunable thresholds for YAML pathology screening.
///
/// Defaults shown per field reflect `PathologyCfg::default()`.

/// Named parameters for the "single slab" heuristic.
///
/// Consider input pathological if the maximum logical line length is at least
/// `min_long_line_bytes` and the total number of newline characters is at most
/// `max_newlines_allowed`.
///
/// Defaults correspond to `SlabCfg::default()`.
#[derive(Clone, Debug)]
pub struct SlabCfg {
    /// Minimum length of the longest logical line in bytes to trigger the
    /// heuristic. Default: 256 KiB.
    pub min_long_line_bytes: usize,
    /// Maximum allowed number of newline characters in the entire input for the
    /// heuristic to trigger. Default: 2.
    pub max_newlines_allowed: usize,
}

impl Default for SlabCfg {
    fn default() -> Self {
        Self {
            min_long_line_bytes: 256 * 1024,
            max_newlines_allowed: 2,
        }
    }
}

/// Named parameters for the alias/anchor ratio heuristic.
///
/// Consider input pathological if the total alias count exceeds
/// `min_aliases`, there is at least one anchor present, and
/// `alias_count > per_anchor_ratio * anchor_count`.
///
/// Defaults correspond to `AliasAnchorCfg::default()`.
#[derive(Clone, Debug)]
pub struct AliasAnchorCfg {
    /// Minimum absolute number of aliases required before the ratio check is
    /// considered. Default: 100.
    pub min_aliases: usize,
    /// Ratio of aliases per each anchor beyond which the input is considered
    /// pathological. Default: 10.
    pub per_anchor_ratio: usize,
}

impl Default for AliasAnchorCfg {
    fn default() -> Self {
        Self {
            min_aliases: 100,
            per_anchor_ratio: 10,
        }
    }
}

/// Tunable thresholds for YAML pathology screening.
///
/// Defaults shown per field reflect `PathologyCfg::default()`.
#[derive(Clone, Debug)]
pub struct PathologyCfg {
    /// Hard cap on accepted input size in bytes. Default: 8 MiB (8 * 1024 * 1024).
    pub max_bytes: usize,
    /// Max logical line length in bytes before bailing (e.g., huge plain/quoted scalar). Default: 512 KiB.
    pub max_line_bytes: usize,
    /// Max run-length for a single repeated byte before bailing. Default: 256 KiB.
    pub max_run_bytes: usize,
    /// Only run heavier entropy/alias checks after this size. Default: 1 MiB.
    pub heavy_after_bytes: usize,
    /// Top-K histogram mass to consider for "low entropy" (alphabet collapse). Default: 4.
    pub low_entropy_topk: usize,
    /// If top-K accounts for at least this percent of the bytes, bail (0–100). Default: 95.
    pub low_entropy_percent: u8,
    /// If the unique alphabet count is ≤ this and size is large, bail. Default: 4.
    pub tiny_alphabet_threshold: usize,
    /// Max allowed increase-only indentation chain length (proxy for nesting). Default: 4096.
    pub max_indent_chain: usize,
    /// Bail if a potential simple key (text before ':' on a line) grows beyond this many bytes. Default: 2048.
    pub max_simple_key_bytes: usize,
    /// Max number of YAML document separators (`---` or `...`) allowed. Default: 1024.
    pub max_documents: usize,
    /// Max total aliases (`*name`) allowed (absolute). Default: 50_000.
    pub max_aliases: usize,
    /// Max aliases per KiB (aliases * 1024 > alias_per_kib * len ⇒ bail). Default: 64.
    pub alias_per_kib: usize,
    /// Max total anchors (`&name`) allowed (absolute). Default: 10_000.
    pub max_anchors: usize,
    /// Max anchors per KiB (anchors * 1024 > anchors_per_kib * len ⇒ bail). Default: 2.
    pub anchors_per_kib: usize,
    /// If true, reject any NUL byte (helps UTF-8-only stacks). Default: true (will trigger false positive on UTF-16 input)
    pub forbid_nul: bool,
    /// Minimum input size required to run pathology checks. Tiny inputs below
    /// this threshold are accepted unconditionally to ensure very short YAML
    /// never trips heuristics. Default: 2048 that is likely below many typical YAML configurations
    /// (it should be no performance issues parsing these)
    pub min_bytes_for_checks: usize,
    /// Parameters for the "single slab" heuristic. See `SlabCfg` for details.
    /// Default: `SlabCfg::default()`.
    pub slab: SlabCfg,
    /// Parameters for the alias/anchor ratio heuristic. See `AliasAnchorCfg`
    /// for details. Default: `AliasAnchorCfg::default()`.
    pub alias_anchor: AliasAnchorCfg,
}

impl Default for PathologyCfg {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,         // 8 MiB
            max_line_bytes: 512 * 1024,         // 512 KiB
            max_run_bytes: 256 * 1024,          // 256 KiB
            heavy_after_bytes: 1 * 1024 * 1024, // 1 MiB
            low_entropy_topk: 4,
            low_entropy_percent: 95,
            tiny_alphabet_threshold: 4,
            max_indent_chain: 4096,     // extremely conservative
            max_simple_key_bytes: 2048, // spec says 1024; keep headroom
            max_documents: 1024,        // guard multi-doc floods
            max_aliases: 50_000,        // generous absolute cap
            alias_per_kib: 64,          // ~1 alias per KiB on average
            max_anchors: 10_000,        // generous absolute cap for anchors
            anchors_per_kib: 2,         // ~2 anchors per KiB on average
            forbid_nul: true,
            min_bytes_for_checks: 2048,
            slab: SlabCfg::default(), // single-slab heuristic defaults
            alias_anchor: AliasAnchorCfg::default(), // (min_aliases, aliases_per_anchor_ratio)
        }
    }
}

/// Return true if `bytes` looks pathological (likely to blow up scan/parse).
///
/// Parameters:
/// - `bytes`: input buffer to screen (raw bytes; assumed UTF‑8 unless you handle BOM).
/// - `cfg`: thresholds for the heuristics (see `PathologyCfg`).
///
/// Returns:
/// - `Some(PathologicalYaml)` ⇒ reject early as pathological, with a specific reason
/// - `None` ⇒ probably OK to hand to the tokenizer

pub fn looks_pathological(bytes: &[u8], cfg: &Option<PathologyCfg>) -> Option<PathologicalYaml> {
    if let Some(cfg) = cfg {
        let bytes_len = bytes.len();
        if bytes_len > cfg.max_bytes {
            return Some(PathologicalYaml {
                reason: PathologyReason::MaxBytes {
                    observed: bytes_len,
                    configured: cfg.max_bytes,
                },
            });
        }
        if bytes_len < cfg.min_bytes_for_checks {
            return None;
        }
        if bytes_len == 0 {
            // Conservatively accept empty
            return None;
        }

        // Precompute alias scaling threshold: alias_count * 1024 > alias_per_kib * bytes_len
        let alias_scaled_threshold = cfg.alias_per_kib.saturating_mul(bytes_len);
        // Precompute anchor scaling threshold: anchor_count * 1024 > anchors_per_kib * bytes_len
        let anchors_scaled_threshold = cfg.anchors_per_kib.saturating_mul(bytes_len);

        // histograms & running state
        let mut counts = [0usize; 256];
        let mut unique = 0usize;

        let mut prev = bytes[0];
        let mut cur_run = 1usize;
        let mut max_run = 1usize;

        let mut current_line = 0usize;
        let mut max_line = 0usize;
        let mut newline_count = 0usize;

        // YAML-specific tallies
        let mut alias_count = 0usize; // '*name'
        let mut anchor_count = 0usize; // '&name'
        let mut doc_count = 0usize;

        // indentation / nesting proxy
        let mut at_line_start = true;
        let mut cur_indent = 0usize;
        let mut prev_indent = 0usize;
        let mut inc_chain = 0usize;
        let mut max_inc_chain = 0usize;

        // simple key (text before ':' at line) length guard
        let mut simple_key_len = 0usize;
        let mut simple_key_open = true; // until we see ':' SP or line end

        // small helpers
        #[inline]
        fn is_name_char(b: u8) -> bool {
            matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'-')
        }

        // For line-start doc separators.
        let mut i = 0usize;
        while i < bytes_len {
            // SAFETY: `i < bytes_len` is maintained by the `while i < bytes_len` loop
            // condition, and `bytes` is not mutated inside the loop. Therefore, the
            // index is always in-bounds and stable for the duration of the loop body.
            let b = unsafe { *bytes.get_unchecked(i) };

            // Histogram / unique
            let c = &mut counts[b as usize];
            if *c == 0 {
                unique += 1;
            }
            *c += 1;

            if cfg.forbid_nul && b == 0 {
                return Some(PathologicalYaml {
                    reason: PathologyReason::ForbidNul {
                        observed: 1,
                        configured: 0,
                    },
                }); // reject early in UTF-8-only mode
            }

            // Run-length
            if b == prev {
                cur_run += 1;
                if cur_run > max_run {
                    max_run = cur_run;
                    if max_run > cfg.max_run_bytes {
                        return Some(PathologicalYaml {
                            reason: PathologyReason::MaxRunBytes {
                                observed: max_run,
                                configured: cfg.max_run_bytes,
                            },
                        });
                    }
                }
            } else {
                prev = b;
                cur_run = 1;
            }

            // Line tracking / indentation / simple-key guard / doc separators
            if at_line_start {
                // Count indentation spaces; tabs at line start are suspicious.
                if b == b' ' {
                    cur_indent += 1;
                } else {
                    // before consuming the first non-space, check for doc sep at column 0
                    if cur_indent == 0 && i + 3 <= bytes_len {
                        // Compare 3 bytes directly without creating a subslice.
                        // SAFETY: The guard `i + 3 <= bytes_len` above ensures that
                        // `i`, `i + 1`, and `i + 2` are all valid indices into `bytes`.
                        // The slice is immutable for the duration of this loop.
                        let b0 = b;
                        let b1 = unsafe { *bytes.get_unchecked(i + 1) };
                        let b2 = unsafe { *bytes.get_unchecked(i + 2) };
                        if (b0 == b'-' && b1 == b'-' && b2 == b'-')
                            || (b0 == b'.' && b1 == b'.' && b2 == b'.')
                        {
                            doc_count += 1;
                        }
                    }
                    // detect monotonic indent increase (proxy for deep nesting)
                    if cur_indent > prev_indent {
                        inc_chain += 1;
                        if inc_chain > max_inc_chain {
                            max_inc_chain = inc_chain;
                        }
                        if max_inc_chain > cfg.max_indent_chain {
                            return Some(PathologicalYaml {
                                reason: PathologyReason::MaxIndentChain {
                                    observed: max_inc_chain,
                                    configured: cfg.max_indent_chain,
                                },
                            });
                        }
                    } else {
                        inc_chain = 0;
                    }
                    prev_indent = cur_indent;

                    // reset simple-key lens for new content
                    simple_key_len = 0;
                    simple_key_open = true;
                    at_line_start = false;
                }
            } else {
                // Simple-key length: grow until colon+space or line break.
                if simple_key_open {
                    if b == b':' {
                        // colon must be followed by space/newline to terminate simple key
                        let next = bytes.get(i + 1).copied().unwrap_or(b'\n');
                        if next == b' ' || next == b'\n' || next == b'\r' {
                            simple_key_open = false; // key closed
                        } else {
                            // "a:1" has no space => not a key; keep counting
                            simple_key_len += 1;
                        }
                    } else if b != b'\n' && b != b'\r' {
                        simple_key_len += 1;
                        if simple_key_len > cfg.max_simple_key_bytes {
                            return Some(PathologicalYaml {
                                reason: PathologyReason::MaxSimpleKeyBytes {
                                    observed: simple_key_len,
                                    configured: cfg.max_simple_key_bytes,
                                },
                            });
                        }
                    }
                }
            }

            // Alias / anchor counters (rough but cheap)
            if b == b'*' {
                if let Some(&nb) = bytes.get(i + 1) {
                    if is_name_char(nb) {
                        alias_count += 1;
                        if alias_count > cfg.max_aliases {
                            return Some(PathologicalYaml {
                                reason: PathologyReason::MaxAliases {
                                    observed: alias_count,
                                    configured: cfg.max_aliases,
                                },
                            });
                        }
                        if alias_count.saturating_mul(1024) > alias_scaled_threshold {
                            return Some(PathologicalYaml {
                                reason: PathologyReason::AliasesPerKiB {
                                    observed_x1024: alias_count.saturating_mul(1024),
                                    configured_xlen: alias_scaled_threshold,
                                },
                            });
                        }
                    }
                }
            } else if b == b'&' {
                if let Some(&nb) = bytes.get(i + 1) {
                    if is_name_char(nb) {
                        anchor_count += 1;
                        if anchor_count > cfg.max_anchors {
                            return Some(PathologicalYaml {
                                reason: PathologyReason::MaxAnchors {
                                    observed: anchor_count,
                                    configured: cfg.max_anchors,
                                },
                            });
                        }
                        if anchor_count.saturating_mul(1024) > anchors_scaled_threshold {
                            return Some(PathologicalYaml {
                                reason: PathologyReason::AnchorsPerKiB {
                                    observed_x1024: anchor_count.saturating_mul(1024),
                                    configured_xlen: anchors_scaled_threshold,
                                },
                            });
                        }
                    }
                }
            }

            // Line / newline
            if b == b'\n' {
                newline_count += 1;
                if current_line > max_line {
                    max_line = current_line;
                }
                current_line = 0;
                at_line_start = true;
                cur_indent = 0;
            } else {
                current_line += 1;
                if current_line > cfg.max_line_bytes {
                    return Some(PathologicalYaml {
                        reason: PathologyReason::MaxLineBytes {
                            observed: current_line,
                            configured: cfg.max_line_bytes,
                        },
                    });
                }
            }

            i += 1;
        }
        if current_line > max_line {
            max_line = current_line;
        }

        // Heavy checks only for large inputs
        if bytes_len >= cfg.heavy_after_bytes {
            // top-K concentration (low-entropy) and tiny alphabet
            let k = cfg.low_entropy_topk.max(1);
            let top_sum = if k <= 8 {
                // stack-based small top-K
                let mut top = [0usize; 8];
                for &cnt in &counts {
                    // insert cnt into top[0..k] if larger than current min
                    // find index of smallest among first k
                    let mut min_idx = 0usize;
                    let mut min_val = top[0];
                    let mut idx = 1usize;
                    while idx < k {
                        if top[idx] < min_val {
                            min_val = top[idx];
                            min_idx = idx;
                        }
                        idx += 1;
                    }
                    if cnt > min_val {
                        top[min_idx] = cnt;
                    }
                }
                let mut s = 0usize;
                let mut idx = 0usize;
                while idx < k {
                    s += top[idx];
                    idx += 1;
                }
                s
            } else {
                let mut top = vec![0usize; k];
                for &cnt in &counts {
                    let mut v = cnt;
                    for t in &mut top {
                        if v > *t {
                            core::mem::swap(&mut v, t);
                        }
                    }
                }
                top.iter().sum()
            };
            let percent = top_sum.saturating_mul(100) / bytes_len;
            if percent >= cfg.low_entropy_percent as usize {
                return Some(PathologicalYaml {
                    reason: PathologyReason::LowEntropyPercent {
                        observed: percent,
                        configured: cfg.low_entropy_percent as usize,
                    },
                });
            }
            if unique <= cfg.tiny_alphabet_threshold {
                return Some(PathologicalYaml {
                    reason: PathologyReason::TinyAlphabet {
                        observed: unique,
                        configured: cfg.tiny_alphabet_threshold,
                    },
                });
            }

            // Few line breaks & very long line: suspicious “single slab”
            if max_line >= cfg.slab.min_long_line_bytes
                && newline_count <= cfg.slab.max_newlines_allowed
            {
                return Some(PathologicalYaml {
                    reason: PathologyReason::SingleSlab {
                        max_line_observed: max_line,
                        min_long_line_bytes: cfg.slab.min_long_line_bytes,
                        newlines_observed: newline_count,
                        max_newlines_allowed: cfg.slab.max_newlines_allowed,
                    },
                });
            }
        }

        // Too many documents?
        if doc_count > cfg.max_documents {
            return Some(PathologicalYaml {
                reason: PathologyReason::MaxDocuments {
                    observed: doc_count,
                    configured: cfg.max_documents,
                },
            });
        }

        // Alias/anchor ratio: lots of aliases, almost no anchors (macro‑like expansion)
        if alias_count > cfg.alias_anchor.min_aliases
            && anchor_count > 0
            && alias_count
                > cfg
                    .alias_anchor
                    .per_anchor_ratio
                    .saturating_mul(anchor_count)
        {
            let threshold_aliases = cfg.alias_anchor.min_aliases;
            return Some(PathologicalYaml {
                reason: PathologyReason::AliasAnchorRatio {
                    aliases_observed: alias_count,
                    anchors_observed: anchor_count,
                    per_anchor_ratio: cfg.alias_anchor.per_anchor_ratio,
                    threshold_aliases,
                },
            });
        }

        // Only print statistics in test configuration and only for non-pathological inputs.
        // This block is reached only if none of the heuristics above triggered an early
        // return. Therefore, the input was NOT detected as pathological.
        #[cfg(test)]
        {
            eprintln!(
                "pathology stats: size={}, unique={}, max_run={}, max_line={}, newlines={}, aliases={}, anchors={}, docs={}",
                bytes_len,
                unique,
                max_run,
                max_line,
                newline_count,
                alias_count,
                anchor_count,
                doc_count
            );
        }
    }

    None
}
