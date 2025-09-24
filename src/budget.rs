//! Streaming YAML budget checker using saphyr-parser (YAML 1.2).
//!
//! This inspects the parser's event stream and enforces simple budgets to
//! avoid pathological inputs 

use std::collections::HashSet;
use std::borrow::Cow;

use saphyr_parser::{Parser, Event, ScanError};

/// Budgets for a streaming YAML scan.
///
/// The defaults are intentionally permissive for typical configuration files
/// while stopping obvious resource-amplifying inputs. Tune these per your
/// application if you regularly process very large YAML streams.
#[derive(Clone, Debug)]
pub struct Budget {
    /// Maximum total parser events (counting every event).
    ///
    /// Default: 1,000,000
    pub max_events: usize,
    /// Maximum number of alias (`*ref`) events allowed.
    ///
    /// Default: 50,000
    pub max_aliases: usize,
    /// Maximal total number of anchors (distinct `&anchor` definitions).
    ///
    /// Default: 50,000
    pub max_anchors: usize,
    /// Maximum structural nesting depth (sequences + mappings).
    ///
    /// Default: 2,000
    pub max_depth: usize,
    /// Maximum number of YAML documents in the stream.
    ///
    /// Default: 1,024
    pub max_documents: usize,
    /// Maximum number of *nodes* (SequenceStart/MappingStart/Scalar).
    ///
    /// Default: 250,000
    pub max_nodes: usize,
    /// Maximum total bytes of scalar contents (sum of `Scalar.value.len()`).
    ///
    /// Default: 67,108,864 (64 MiB)
    pub max_total_scalar_bytes: usize,
    /// If `true`, enforce the alias/anchor heuristic.
    ///
    /// The heuristic flags inputs that use an excessive number of aliases
    /// relative to the number of defined anchors.
    ///
    /// Default: true
    pub enforce_alias_anchor_ratio: bool,
    /// Minimum number of aliases required before the alias/anchor ratio
    /// heuristic is evaluated. This avoids tiny-input false positives.
    ///
    /// Default: 100
    pub alias_anchor_min_aliases: usize,
    /// Multiplier used for the alias/anchor ratio heuristic. A breach occurs
    /// when `aliases > alias_anchor_ratio_multiplier * anchors` (after
    /// scanning), once [`Budget::alias_anchor_min_aliases`] is met.
    ///
    /// Default: 10
    pub alias_anchor_ratio_multiplier: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_events: 1_000_000,              // plenty for normal configs
            max_aliases: 50_000,                // liberal absolute cap
            max_anchors: 50_000,    
            max_depth: 2_000,                   // protects stack/CPU
            max_documents: 1_024,               // doc separator storms
            max_nodes: 250_000,                 // sequences + maps + scalars
            max_total_scalar_bytes: 64 * 1024 * 1024, // 64 MiB of scalar text
            enforce_alias_anchor_ratio: true,
            alias_anchor_min_aliases: 100,
            alias_anchor_ratio_multiplier: 10,
        }
    }
}

/// What tripped the budget (if anything).
#[derive(Clone, Debug)]
pub enum BudgetBreach {
    /// The total number of parser events exceeded [`Budget::max_events`].
    Events {
        /// Total events observed at the moment of the breach.
        events: usize,
    },

    /// The number of alias events (`*ref`) exceeded [`Budget::max_aliases`].
    Aliases {
        /// Total alias events observed at the moment of the breach.
        aliases: usize,
    },

    /// The number of distinct anchors defined exceeded [`Budget::max_anchors`].
    Anchors {
        /// Total distinct anchors observed at the moment of the breach.
        anchors: usize,
    },

    /// The structural nesting depth exceeded [`Budget::max_depth`].
    ///
    /// Depth counts nested `SequenceStart` and `MappingStart` events.
    Depth {
        /// Maximum depth reached when the breach occurred.
        depth: usize,
    },

    /// The number of YAML documents exceeded [`Budget::max_documents`].
    Documents {
        /// Total documents observed at the moment of the breach.
        documents: usize,
    },

    /// The number of nodes exceeded [`Budget::max_nodes`].
    ///
    /// Nodes are `SequenceStart`, `MappingStart`, and `Scalar` events.
    Nodes {
        /// Total nodes observed at the moment of the breach.
        nodes: usize,
    },

    /// The cumulative size of scalar contents exceeded [`Budget::max_total_scalar_bytes`].
    ScalarBytes {
        /// Sum of `Scalar.value.len()` over all scalars seen so far.
        total_scalar_bytes: usize,
    },

    /// The ratio of aliases to defined anchors is excessive.
    ///
    /// Triggered when [`Budget::enforce_alias_anchor_ratio`] is true and
    /// `aliases > alias_anchor_ratio_multiplier × anchors` (after scanning),
    /// once `aliases >= alias_anchor_min_aliases` to avoid tiny-input
    /// false positives.
    AliasAnchorRatio {
        /// Total alias events seen.
        aliases: usize,
        /// Total distinct anchors defined (by id) in the input.
        anchors: usize,
    },

    /// Unbalanced structure: a closing event was encountered without a matching
    /// opening event (depth underflow). Indicates malformed or truncated input.
    SequenceUnbalanced,
}

/// Summary of the scan (even if no breach).
#[derive(Clone, Debug, Default)]
pub struct BudgetReport {
    /// `Some(..)` if a limit was exceeded; `None` if all budgets were respected.
    pub breached: Option<BudgetBreach>,

    /// Total number of parser events observed.
    pub events: usize,

    /// Total number of alias events (`*ref`).
    pub aliases: usize,

    /// Total number of distinct anchors that were defined (by id).
    pub anchors: usize,

    /// Total number of YAML documents in the stream.
    pub documents: usize,

    /// Total number of nodes encountered (scalars + sequence starts + mapping starts).
    pub nodes: usize,

    /// Maximum structural nesting depth reached at any point in the stream.
    pub max_depth: usize,

    /// Sum of bytes across all scalar values (`Scalar.value.len()`), saturating on overflow.
    pub total_scalar_bytes: usize,
}

/// Check an input `&str` against the given `Budget`.
///
/// Parameters:
/// - `input`: YAML text (UTF-8). If you accept non-UTF-8, transcode before calling.
/// - `budget`: limits to enforce (see [`Budget`]).
///
/// Returns:
/// - `Ok(report)` — `report.breached.is_none()` means **within budget**.
///   If `report.breached.is_some()`, you should **reject** the input.
/// - `Err(ScanError)` — scanning (lexing/parsing) failed.
///
/// Note:
/// - This is **streaming** and does not allocate a DOM.
/// - Depth counts nested `SequenceStart` and `MappingStart`.
pub fn check_yaml_budget(input: &str, budget: &Budget) -> Result<BudgetReport, ScanError> {
    let mut parser = Parser::new_from_str(input);

    let mut report = BudgetReport::default();
    let mut depth: usize = 0;

    // Track anchors that were actually defined (IDs from starts/scalars).
    // saphyr-parser attaches an "anchor id" (usize) to Scalar/SequenceStart/MappingStart.
    // The Alias event carries an anchor id it references.
    let mut defined_anchors: HashSet<usize> = HashSet::with_capacity(256);

    // Helper: early-return with a breach
    macro_rules! breach {
        ($kind:expr) => {{
            report.breached = Some($kind);
            return Ok(report);
        }};
    }

    // Iterate the event stream; this avoids implementing EventReceiver.
    while let Some(item) = parser.next() {
        let (ev, _span) = item?; // propagate ScanError

        report.events += 1;
        if report.events > budget.max_events {
            breach!(BudgetBreach::Events { events: report.events });
        }

        match ev {
            Event::StreamStart => {}
            Event::StreamEnd => {}
            Event::DocumentStart(_explicit) => {
                report.documents += 1;
                if report.documents > budget.max_documents {
                    breach!(BudgetBreach::Documents { documents: report.documents });
                }
            }
            Event::DocumentEnd => {}

            Event::Alias(anchor_id) => {
                report.aliases += 1;
                if report.aliases > budget.max_aliases {
                    breach!(BudgetBreach::Aliases { aliases: report.aliases });
                }
                // alias/anchor ratio checked after the loop with totals
                let _ = anchor_id; // we don't need to resolve it here
            }

            Event::Scalar(value, _style, anchor_id, _tag_opt) => {
                report.nodes += 1;
                if report.nodes > budget.max_nodes {
                    breach!(BudgetBreach::Nodes { nodes: report.nodes });
                }
                // Count scalar bytes
                let len = match value {
                    Cow::Borrowed(s) => s.len(),
                    Cow::Owned(s) => s.len(),
                };
                report.total_scalar_bytes = report.total_scalar_bytes.saturating_add(len);
                if report.total_scalar_bytes > budget.max_total_scalar_bytes {
                    breach!(BudgetBreach::ScalarBytes { total_scalar_bytes: report.total_scalar_bytes });
                }
                if anchor_id != 0 {
                    if defined_anchors.insert(anchor_id) {
                        if defined_anchors.len() > budget.max_anchors {
                            breach!(BudgetBreach::Anchors { anchors: defined_anchors.len() });
                        }
                    }
                }
            }

            Event::SequenceStart(anchor_id, _tag_opt) => {
                report.nodes += 1;
                if report.nodes > budget.max_nodes {
                    breach!(BudgetBreach::Nodes { nodes: report.nodes });
                }
                depth += 1;
                if depth > report.max_depth {
                    report.max_depth = depth;
                }
                if report.max_depth > budget.max_depth {
                    breach!(BudgetBreach::Depth { depth: report.max_depth });
                }
                if anchor_id != 0 {
                    if defined_anchors.insert(anchor_id) {
                        if defined_anchors.len() > budget.max_anchors {
                            breach!(BudgetBreach::Anchors { anchors: defined_anchors.len() });
                        }
                    }
                }
            }
            Event::SequenceEnd => {
                if let Some(new_depth) = depth.checked_sub(1) {
                    depth = new_depth;
                } else {
                    breach!(BudgetBreach::SequenceUnbalanced);
                }
            }

            Event::MappingStart(anchor_id, _tag_opt) => {
                report.nodes += 1;
                if report.nodes > budget.max_nodes {
                    breach!(BudgetBreach::Nodes { nodes: report.nodes });
                }
                depth += 1;
                if depth > report.max_depth {
                    report.max_depth = depth;
                }
                if report.max_depth > budget.max_depth {
                    breach!(BudgetBreach::Depth { depth: report.max_depth });
                }
                if anchor_id != 0 {
                    if defined_anchors.insert(anchor_id) {
                        if defined_anchors.len() > budget.max_anchors {
                            breach!(BudgetBreach::Anchors { anchors: defined_anchors.len() });
                        }
                    }
                }
            }
            Event::MappingEnd => {
                if let Some(new_depth) = depth.checked_sub(1) {
                    depth = new_depth;
                } else {
                    breach!(BudgetBreach::SequenceUnbalanced);
                }
            }

            Event::Nothing => {}
        }
    }

    report.anchors = defined_anchors.len();

    if budget.enforce_alias_anchor_ratio && report.aliases >= budget.alias_anchor_min_aliases {
        // Heuristic: too many aliases compared to anchors hints at macro-like expansion.
        if report.anchors == 0 || report.aliases > budget.alias_anchor_ratio_multiplier * report.anchors {
            breach!(BudgetBreach::AliasAnchorRatio {
                aliases: report.aliases,
                anchors: report.anchors,
            });
        }
    }

    Ok(report)
}

/// Convenience wrapper that returns `true` if the YAML **exceeds** any budget.
///
/// Parameters:
/// - `input`: YAML text (UTF-8).
/// - `budget`: limits to enforce.
///
/// Returns:
/// - `Ok(true)` if a budget was exceeded (reject).
/// - `Ok(false)` if within budget.
/// - `Err(ScanError)` on parser error.
pub fn exceeds_yaml_budget(input: &str, budget: &Budget) -> Result<bool, ScanError> {
    let report = check_yaml_budget(input, budget)?;
    Ok(report.breached.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_yaml_ok() {
        let b = Budget::default();
        let y = "a: [1, 2, 3]\n";
        let r = check_yaml_budget(y, &b).unwrap();
        assert!(r.breached.is_none());
        assert_eq!(r.documents, 1);
        assert_eq!(r.nodes > 0, true);
    }

    #[test]
    fn alias_bomb_trips_alias_limit() {
        // A toy alias-bomb-ish input (not huge, just to exercise the check).
        let y = r#"root: &A [1, 2]
a: *A
b: *A
c: *A
d: *A
e: *A
"#;

        let mut b = Budget::default();
        b.max_aliases = 3; // set a tiny limit for the test

        let rep = check_yaml_budget(y, &b).unwrap();
        assert!(matches!(rep.breached, Some(BudgetBreach::Aliases{ .. })));
    }

    #[test]
    fn deep_nesting_trips_depth() {
        let mut y = String::new();
        // Keep nesting below saphyr's internal recursion ceiling to ensure
        // the budget check, not the parser, trips first.
        for _ in 0..200 {
            y.push('[');
        }
        for _ in 0..200 {
            y.push(']');
        }

        let mut b = Budget::default();
        b.max_depth = 150;

        let rep = check_yaml_budget(&y, &b).unwrap();
        assert!(matches!(rep.breached, Some(BudgetBreach::Depth{ .. })));
    }

    #[test]
    fn anchors_limit_trips() {
        // Three distinct anchors defined on scalar nodes
        let y = "a: &A 1\nb: &B 2\nc: &C 3\n";
        let mut b = Budget::default();
        b.max_anchors = 2;
        let rep = check_yaml_budget(y, &b).unwrap();
        assert!(matches!(rep.breached, Some(BudgetBreach::Anchors { anchors: 3 })));
    }
}
