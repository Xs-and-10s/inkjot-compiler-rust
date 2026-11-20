//! The story as a graph: every passage, and every way from one to another.
//!
//! For the platform's path tree (the Linker's reading, drawn over the whole
//! story) and for the Inker's path analytics. It is worked out from the IR,
//! so it adds the edges the IR only implies: a `next:`, the ways of a map.
//! Forks are edges too, marked as forks, so the platform decides who may
//! see them.

use super::{f, Story};
use crate::json::Value;

/// One passage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    /// `title: "…"`, when the passage has one.
    pub title: Option<String>,
    pub tags: Vec<String>,
}

/// One way from a passage to another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// `link`, `fork`, `next` or `way`.
    pub kind: &'static str,
    /// The Link's id, for a Link or a fork; the way's, for a way.
    pub id: Option<String>,
    /// The Link's label as plain text; a value in braces reads `…`.
    pub label: String,
    /// For a fork: the offered Link it stands in for.
    pub stands_in_for: Option<String>,
    /// The edge has a condition (`~ when`, `next: x when …`, `open when`).
    pub conditional: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryMap {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Story {
    /// The whole story as a graph, in source order.
    pub fn map(&self) -> StoryMap {
        let mut map = StoryMap::default();
        for (name, p) in self.ordered() {
            if p.tags.iter().any(|t| t == "test") {
                continue;
            }
            map.nodes.push(Node {
                id: name.clone(),
                title: p.title.clone(),
                tags: p.tags.clone(),
            });
            for stmt in &p.header {
                if f(stmt, "setting").as_str() == Some("next") {
                    if let Some(to) = f(stmt, "value").as_str() {
                        map.edges.push(Edge {
                            from: name.clone(),
                            to: to.to_string(),
                            kind: "next",
                            id: None,
                            label: String::new(),
                            stands_in_for: None,
                            conditional: *f(stmt, "when") != Value::Null,
                        });
                    }
                }
            }
            for node in &p.body {
                if f(node, "type").as_str() != Some("link") {
                    continue;
                }
                let target = f(node, "target").as_str().unwrap_or("");
                let to = match target {
                    "rewind" | "" => continue,
                    "here" => name.as_str(),
                    t => t,
                };
                let inquest = node.get("inquest");
                map.edges.push(Edge {
                    from: name.clone(),
                    to: to.to_string(),
                    kind: if inquest.is_some() { "fork" } else { "link" },
                    id: f(node, "id").as_str().map(str::to_string),
                    label: plain_label(f(node, "label")),
                    stands_in_for: inquest
                        .and_then(|i| f(i, "stands_in_for").as_str())
                        .map(str::to_string),
                    conditional: *f(node, "when") != Value::Null
                        || *f(node, "hidden_until") != Value::Null,
                });
            }
        }
        // A map's ways join its places' passages, both ways unless one way.
        for (way, from, to) in &self.ways {
            if self.passages.contains_key(from) && self.passages.contains_key(to) {
                map.edges.push(Edge {
                    from: from.clone(),
                    to: to.clone(),
                    kind: "way",
                    id: Some(way.clone()),
                    label: String::new(),
                    stands_in_for: None,
                    conditional: *self.field(way, "open_when") != Value::Null,
                });
            }
        }
        map
    }

    /// The passages in the order the source has them.
    fn ordered(&self) -> Vec<(&String, &super::Passage)> {
        let mut all: Vec<(&String, &super::Passage)> = self.passages.iter().collect();
        all.sort_by_key(|(name, _)| self.order.iter().position(|o| o == *name));
        all
    }
}

/// A label's parts as plain text: words kept, styles dropped, values `…`.
fn plain_label(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Arr(parts) => parts.iter().map(plain_label).collect(),
        Value::Obj(_) if v.get("parts").is_some() => plain_label(f(v, "parts")),
        Value::Obj(_) => "…".to_string(),
        _ => String::new(),
    }
}
