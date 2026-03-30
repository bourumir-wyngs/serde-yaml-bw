//! Streaming YAML budget checker using saphyr-parser (YAML 1.2).
//!
//! This inspects the parser's event stream and enforces simple budgets to
//! avoid pathological inputs 

use std::borrow::Cow;
use std::collections::HashSet;

use saphyr_parser::{Event, Parser, ScanError};

fn fallback_budget_input(input: &str) -> Option<&str> {
    let mut offset = 0;

    for line in input.split_inclusive('\n') {
        let line_end = offset + line.len();
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(rest) = trimmed.strip_prefix("...") {
            let rest = rest.trim_start_matches([' ', '\t']);
            if rest.is_empty() || rest.starts_with('#') {
                let tail = &input[line_end..];
                let next = tail.trim_start_matches([' ', '\t', '\r', '\n']);
                if next.is_empty() || next.starts_with("---") {
                    return None;
                }
                return Some(&input[..line_end]);
            }
        }
        offset = line_end;
    }

    None
}

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

#[derive(Clone, Copy, Debug)]
pub(crate) enum BudgetEvent<'a, A> {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias(A),
    Scalar {
        anchor: Option<A>,
        scalar_bytes: usize,
    },
    SequenceStart {
        anchor: Option<A>,
    },
    SequenceEnd,
    MappingStart {
        anchor: Option<A>,
    },
    MappingEnd,
    Nothing,
    #[allow(dead_code)]
    _Marker(std::marker::PhantomData<&'a A>),
}

#[derive(Clone, Debug)]
struct BudgetScope<A> {
    aliases: usize,
    nodes: usize,
    max_depth: usize,
    total_scalar_bytes: usize,
    anchors: HashSet<A>,
}

impl<A> Default for BudgetScope<A> {
    fn default() -> Self {
        Self {
            aliases: 0,
            nodes: 0,
            max_depth: 0,
            total_scalar_bytes: 0,
            anchors: HashSet::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BudgetTracker<A> {
    budget: Budget,
    report: BudgetReport,
    depth: usize,
    scope: BudgetScope<A>,
}

impl<A> BudgetTracker<A>
where
    A: Eq + std::hash::Hash,
{
    pub(crate) fn new(budget: &Budget) -> Self {
        Self {
            budget: budget.clone(),
            report: BudgetReport::default(),
            depth: 0,
            scope: BudgetScope {
                anchors: HashSet::with_capacity(256),
                ..BudgetScope::default()
            },
        }
    }

    fn reset_scope(&mut self) {
        self.depth = 0;
        self.scope.aliases = 0;
        self.scope.nodes = 0;
        self.scope.max_depth = 0;
        self.scope.total_scalar_bytes = 0;
        self.scope.anchors.clear();
    }

    fn breach(&mut self, breach: BudgetBreach) -> BudgetBreach {
        self.report.breached = Some(breach.clone());
        breach
    }

    fn check_alias_anchor_ratio(&mut self) -> Result<(), BudgetBreach> {
        if self.budget.enforce_alias_anchor_ratio
            && self.scope.aliases >= self.budget.alias_anchor_min_aliases
        {
            let anchors = self.scope.anchors.len();
            if anchors == 0 || self.scope.aliases > self.budget.alias_anchor_ratio_multiplier * anchors
            {
                return Err(self.breach(BudgetBreach::AliasAnchorRatio {
                    aliases: self.scope.aliases,
                    anchors,
                }));
            }
        }

        Ok(())
    }

    fn finish_document(&mut self) -> Result<(), BudgetBreach> {
        self.check_alias_anchor_ratio()?;
        self.report.aliases = self.scope.aliases;
        self.report.anchors = self.scope.anchors.len();
        self.report.nodes = self.scope.nodes;
        self.report.max_depth = self.scope.max_depth;
        self.report.total_scalar_bytes = self.scope.total_scalar_bytes;
        self.reset_scope();
        Ok(())
    }

    pub(crate) fn observe<'a>(&mut self, event: BudgetEvent<'a, A>) -> Result<(), BudgetBreach> {
        self.report.events += 1;
        if self.report.events > self.budget.max_events {
            return Err(self.breach(BudgetBreach::Events {
                events: self.report.events,
            }));
        }

        match event {
            BudgetEvent::StreamStart | BudgetEvent::StreamEnd | BudgetEvent::Nothing => {}
            BudgetEvent::DocumentStart => {
                self.report.documents += 1;
                if self.report.documents > self.budget.max_documents {
                    return Err(self.breach(BudgetBreach::Documents {
                        documents: self.report.documents,
                    }));
                }
            }
            BudgetEvent::DocumentEnd => {
                self.finish_document()?;
            }
            BudgetEvent::Alias(_anchor) => {
                self.scope.aliases += 1;
                self.report.aliases = self.scope.aliases;
                if self.scope.aliases > self.budget.max_aliases {
                    return Err(self.breach(BudgetBreach::Aliases {
                        aliases: self.scope.aliases,
                    }));
                }
            }
            BudgetEvent::Scalar {
                anchor,
                scalar_bytes,
            } => {
                self.scope.nodes += 1;
                self.report.nodes = self.scope.nodes;
                if self.scope.nodes > self.budget.max_nodes {
                    return Err(self.breach(BudgetBreach::Nodes {
                        nodes: self.scope.nodes,
                    }));
                }

                self.scope.total_scalar_bytes =
                    self.scope.total_scalar_bytes.saturating_add(scalar_bytes);
                self.report.total_scalar_bytes = self.scope.total_scalar_bytes;
                if self.scope.total_scalar_bytes > self.budget.max_total_scalar_bytes {
                    return Err(self.breach(BudgetBreach::ScalarBytes {
                        total_scalar_bytes: self.scope.total_scalar_bytes,
                    }));
                }

                if let Some(anchor) = anchor {
                    if self.scope.anchors.insert(anchor) {
                        self.report.anchors = self.scope.anchors.len();
                        if self.scope.anchors.len() > self.budget.max_anchors {
                            return Err(self.breach(BudgetBreach::Anchors {
                                anchors: self.scope.anchors.len(),
                            }));
                        }
                    }
                }
            }
            BudgetEvent::SequenceStart { anchor } | BudgetEvent::MappingStart { anchor } => {
                self.scope.nodes += 1;
                self.report.nodes = self.scope.nodes;
                if self.scope.nodes > self.budget.max_nodes {
                    return Err(self.breach(BudgetBreach::Nodes {
                        nodes: self.scope.nodes,
                    }));
                }

                self.depth += 1;
                if self.depth > self.scope.max_depth {
                    self.scope.max_depth = self.depth;
                    self.report.max_depth = self.scope.max_depth;
                }
                if self.scope.max_depth > self.budget.max_depth {
                    return Err(self.breach(BudgetBreach::Depth {
                        depth: self.scope.max_depth,
                    }));
                }

                if let Some(anchor) = anchor {
                    if self.scope.anchors.insert(anchor) {
                        self.report.anchors = self.scope.anchors.len();
                        if self.scope.anchors.len() > self.budget.max_anchors {
                            return Err(self.breach(BudgetBreach::Anchors {
                                anchors: self.scope.anchors.len(),
                            }));
                        }
                    }
                }
            }
            BudgetEvent::SequenceEnd | BudgetEvent::MappingEnd => {
                if let Some(new_depth) = self.depth.checked_sub(1) {
                    self.depth = new_depth;
                } else {
                    return Err(self.breach(BudgetBreach::SequenceUnbalanced));
                }
            }
            BudgetEvent::_Marker(_) => unreachable!(),
        }

        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<BudgetReport, BudgetBreach> {
        if self.report.breached.is_none() && self.report.documents > 0 && self.depth == 0 {
            if self.scope.aliases > 0
                || self.scope.nodes > 0
                || self.scope.total_scalar_bytes > 0
                || !self.scope.anchors.is_empty()
                || self.scope.max_depth > 0
            {
                self.finish_document()?;
            }
        }
        Ok(self.report)
    }
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
    let mut tracker = BudgetTracker::<usize>::new(budget);

    // Iterate the event stream; this avoids implementing EventReceiver.
    while let Some(item) = parser.next() {
        let (ev, _span) = match item {
            Ok(item) => item,
            Err(err) => {
                if let Some(prefix) = fallback_budget_input(input) {
                    return check_yaml_budget(prefix, budget);
                }
                return Err(err);
            }
        };

        let budget_event = match ev {
            Event::StreamStart => BudgetEvent::StreamStart,
            Event::StreamEnd => BudgetEvent::StreamEnd,
            Event::DocumentStart(_explicit) => BudgetEvent::DocumentStart,
            Event::DocumentEnd => BudgetEvent::DocumentEnd,
            Event::Alias(anchor_id) => BudgetEvent::Alias(anchor_id),
            Event::Scalar(value, _style, anchor_id, _tag_opt) => BudgetEvent::Scalar {
                anchor: (anchor_id != 0).then_some(anchor_id),
                scalar_bytes: match value {
                    Cow::Borrowed(s) => s.len(),
                    Cow::Owned(s) => s.len(),
                },
            },
            Event::SequenceStart(anchor_id, _tag_opt) => BudgetEvent::SequenceStart {
                anchor: (anchor_id != 0).then_some(anchor_id),
            },
            Event::SequenceEnd => BudgetEvent::SequenceEnd,
            Event::MappingStart(anchor_id, _tag_opt) => BudgetEvent::MappingStart {
                anchor: (anchor_id != 0).then_some(anchor_id),
            },
            Event::MappingEnd => BudgetEvent::MappingEnd,
            Event::Nothing => BudgetEvent::Nothing,
        };

        if let Err(breach) = tracker.observe(budget_event) {
            let mut report = tracker.finish().unwrap_or_else(|_| BudgetReport::default());
            report.breached = Some(breach);
            return Ok(report);
        }
    }

    match tracker.finish() {
        Ok(report) => Ok(report),
        Err(breach) => Ok(BudgetReport {
            breached: Some(breach),
            ..BudgetReport::default()
        }),
    }
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

    #[test]
    fn explicit_end_marker_allows_ignoring_trailing_garbage() {
        let yaml = "---\na: 1\n...\n!!! trailing garbage\n";
        let report = check_yaml_budget(yaml, &Budget::default()).unwrap();
        assert!(report.breached.is_none());
        assert_eq!(report.documents, 1);
    }

    #[test]
    fn explicit_end_marker_keeps_following_documents_visible() {
        let yaml = "---\na: 1\n...\n---\nb: 2\n";
        let report = check_yaml_budget(yaml, &Budget::default()).unwrap();
        assert!(report.breached.is_none());
        assert_eq!(report.documents, 2);
    }

    #[test]
    fn non_document_limits_reset_per_document() {
        let yaml = "---\na: 1\nb: 2\n---\nc: 3\nd: 4\n";
        let mut budget = Budget::default();
        budget.max_nodes = 5;

        let report = check_yaml_budget(yaml, &budget).unwrap();
        assert!(report.breached.is_none());
        assert_eq!(report.documents, 2);
        assert_eq!(report.nodes, 5);
    }

    #[test]
    fn document_count_remains_stream_wide() {
        let yaml = "---\na: 1\n---\nb: 2\n";
        let mut budget = Budget::default();
        budget.max_documents = 1;

        let report = check_yaml_budget(yaml, &budget).unwrap();
        assert!(matches!(
            report.breached,
            Some(BudgetBreach::Documents { documents: 2 })
        ));
    }
}
