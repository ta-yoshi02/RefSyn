use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Operation {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub is_literal: bool,
    #[serde(default)]
    pub label: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VisGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
