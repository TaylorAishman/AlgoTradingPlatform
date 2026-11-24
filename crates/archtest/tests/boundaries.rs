//! Policy tests for crate dependency boundaries (R-004).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

/// Normal (non-dev, non-build) dependency edges by package name, plus the workspace member names.
struct Graph {
    edges: BTreeMap<String, BTreeSet<String>>,
    members: BTreeSet<String>,
}

fn graph() -> Graph {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(root)
        .output()
        .expect("run cargo metadata");
    assert!(out.status.success(), "cargo metadata failed");
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).expect("metadata JSON");

    let mut id_to_name = BTreeMap::new();
    for pkg in meta["packages"].as_array().expect("packages array") {
        id_to_name.insert(
            pkg["id"].as_str().expect("package id").to_string(),
            pkg["name"].as_str().expect("package name").to_string(),
        );
    }
    let member_ids: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .expect("workspace_members")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    let members = member_ids
        .iter()
        .filter_map(|id| id_to_name.get(*id).cloned())
        .collect();

    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in meta["resolve"]["nodes"].as_array().expect("resolve nodes") {
        let name = id_to_name[node["id"].as_str().expect("node id")].clone();
        let set = edges.entry(name).or_default();
        for dep in node["deps"].as_array().expect("deps") {
            let normal = dep["dep_kinds"]
                .as_array()
                .expect("dep_kinds")
                .iter()
                .any(|k| k["kind"].is_null());
            if normal {
                set.insert(id_to_name[dep["pkg"].as_str().expect("dep id")].clone());
            }
        }
    }
    Graph { edges, members }
}

impl Graph {
    fn require(&self, name: &str) {
        assert!(
            self.members.contains(name),
            "{name} is not a workspace member"
        );
    }

    fn direct(&self, name: &str) -> BTreeSet<String> {
        self.require(name);
        self.edges.get(name).cloned().unwrap_or_default()
    }

    fn closure(&self, name: &str) -> BTreeSet<String> {
        self.require(name);
        let mut seen = BTreeSet::new();
        let mut stack = vec![name.to_string()];
        while let Some(n) = stack.pop() {
            for d in self.edges.get(&n).into_iter().flatten() {
                if seen.insert(d.clone()) {
                    stack.push(d.clone());
                }
            }
        }
        seen
    }

    fn workspace_deps(&self, name: &str) -> BTreeSet<String> {
        self.direct(name)
            .into_iter()
            .filter(|d| self.members.contains(d))
            .collect()
    }
}

// R-004: the five new crates exist as workspace members.
#[test]
fn strategy_portfolio_risk_engine_validate_are_members() {
    let g = graph();
    for name in [
        "pt-strategy",
        "pt-portfolio",
        "pt-risk",
        "pt-engine",
        "pt-validate",
    ] {
        g.require(name);
    }
}

// R-004: pt-strategy cannot reach a broker, a gate, a database or the network.
#[test]
fn strategy_closure_excludes_brokers_stores_and_http() {
    let g = graph();
    let closure = g.closure("pt-strategy");
    for forbidden in [
        "pt-alpaca",
        "pt-store",
        "pt-live",
        "pt-gate",
        "ureq",
        "rusqlite",
    ] {
        assert!(
            !closure.contains(forbidden),
            "pt-strategy reaches {forbidden}"
        );
    }
}

// R-004: pt-vector depends on pt-core and nothing else in the workspace.
#[test]
fn vector_depends_on_exactly_core() {
    let g = graph();
    let want: BTreeSet<String> = ["pt-core".to_string()].into();
    assert_eq!(g.workspace_deps("pt-vector"), want);
}

// R-004: pt-stats has no workspace dependency.
#[test]
fn stats_has_no_workspace_dependency() {
    let g = graph();
    assert_eq!(g.workspace_deps("pt-stats"), BTreeSet::new());
}

// R-004: pt-engine never reaches pt-vector.
#[test]
fn engine_closure_excludes_vector() {
    let g = graph();
    assert!(!g.closure("pt-engine").contains("pt-vector"));
}
