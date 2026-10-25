use crate::record::Record;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub struct Node {
    pub children: BTreeMap<String, Node>,
    pub total: usize,
}

pub fn bare(name: &str) -> &str {
    name.trim_start_matches("*.").trim_end_matches('.')
}

pub fn under(name: &str, domain: &str) -> bool {
    let name = bare(name);
    name == domain || name.ends_with(&format!(".{domain}"))
}

fn ancestors(name: &str) -> Vec<String> {
    let labels: Vec<&str> = name.split('.').collect();
    (0..labels.len())
        .rev()
        .map(|i| labels[i..].join("."))
        .collect()
}

pub fn build(records: &[Record]) -> Node {
    let mut root = Node::default();
    for r in records {
        let mut counted: BTreeSet<String> = BTreeSet::new();
        for name in &r.dns_names {
            let name = bare(name).to_lowercase();
            if name.is_empty() {
                continue;
            }
            let mut node = &mut root;
            for full in ancestors(&name) {
                node = node.children.entry(full.clone()).or_default();
                if counted.insert(full) {
                    node.total += 1;
                }
            }
        }
    }
    root
}

pub fn find<'a>(root: &'a Node, name: &str) -> Option<&'a Node> {
    let mut node = root;
    for full in ancestors(name) {
        node = node.children.get(&full)?;
    }
    Some(node)
}

pub fn label(full: &str) -> &str {
    full.split('.').next().unwrap_or(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(names: &[&str]) -> Record {
        serde_json::from_value(serde_json::json!({
            "tbs_sha256": "t", "cert_sha256": "c", "zone": "x", "log_uri": "",
            "entry_index": 0, "discovered_at": 0, "dns_names": names,
        }))
        .unwrap()
    }

    #[test]
    fn names_fold_into_a_tree_counted_once_per_cert() {
        let records = vec![
            rec(&["*.estrogen.coffee", "estrogen.coffee"]),
            rec(&["dash.hive.hayl.in"]),
            rec(&["hayl.in", "auth.hayl.in"]),
        ];
        let root = build(&records);
        assert_eq!(
            root.children.keys().collect::<Vec<_>>(),
            vec!["coffee", "in"]
        );
        assert_eq!(find(&root, "estrogen.coffee").unwrap().total, 1);
        assert_eq!(find(&root, "hayl.in").unwrap().total, 2);
        assert_eq!(find(&root, "hive.hayl.in").unwrap().total, 1);
        assert!(
            find(&root, "dash.hive.hayl.in")
                .unwrap()
                .children
                .is_empty()
        );
        assert!(find(&root, "nope.example").is_none());
    }

    #[test]
    fn under_matches_the_name_its_subdomains_and_wildcards() {
        assert!(under("hayl.in", "hayl.in"));
        assert!(under("dash.hive.hayl.in", "hayl.in"));
        assert!(under("*.hayl.in", "hayl.in"));
        assert!(!under("nothayl.in", "hayl.in"));
        assert_eq!(label("dash.hive.hayl.in"), "dash");
    }
}
