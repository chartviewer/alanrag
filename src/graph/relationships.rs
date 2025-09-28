use super::builder::{GraphNode, GraphEdge, EdgeType};
use std::collections::HashMap;

#[derive(PartialEq, PartialOrd)]
struct OrderedFloat(f32);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

pub struct RelationshipAnalyzer {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
}

impl RelationshipAnalyzer {
    pub fn new(nodes: HashMap<String, GraphNode>, edges: Vec<GraphEdge>) -> Self {
        Self { nodes, edges }
    }

    pub fn find_shortest_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        let mut distances = HashMap::new();
        let mut previous = HashMap::new();
        let mut unvisited = std::collections::BinaryHeap::new();

        // Initialize distances
        for node_id in self.nodes.keys() {
            distances.insert(node_id.clone(), f32::INFINITY);
        }
        distances.insert(from.to_string(), 0.0);

        unvisited.push(std::cmp::Reverse((OrderedFloat(0.0), from.to_string())));

        while let Some(std::cmp::Reverse((current_distance, current_node))) = unvisited.pop() {
            let current_distance = current_distance.0;
            if current_node == to {
                break;
            }

            if current_distance > *distances.get(&current_node).unwrap_or(&f32::INFINITY) {
                continue;
            }

            for edge in &self.edges {
                let neighbor = if edge.from == current_node {
                    &edge.to
                } else if edge.to == current_node {
                    &edge.from
                } else {
                    continue;
                };

                let distance = current_distance + (1.0 / edge.weight); // Invert weight for shortest path

                if distance < *distances.get(neighbor).unwrap_or(&f32::INFINITY) {
                    distances.insert(neighbor.clone(), distance);
                    previous.insert(neighbor.clone(), current_node.clone());
                    unvisited.push(std::cmp::Reverse((OrderedFloat(distance), neighbor.clone())));
                }
            }
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut current = to.to_string();

        while let Some(prev) = previous.get(&current) {
            path.push(current.clone());
            current = prev.clone();
        }

        if current == from {
            path.push(from.to_string());
            path.reverse();
            Some(path)
        } else {
            None
        }
    }

    pub fn get_related_by_type(&self, node_id: &str, edge_type: EdgeType) -> Vec<String> {
        let mut related = Vec::new();

        for edge in &self.edges {
            if std::mem::discriminant(&edge.edge_type) == std::mem::discriminant(&edge_type) {
                if edge.from == node_id {
                    related.push(edge.to.clone());
                } else if edge.to == node_id {
                    related.push(edge.from.clone());
                }
            }
        }

        related
    }

    pub fn calculate_centrality(&self, node_id: &str) -> f32 {
        let mut centrality = 0.0;

        for edge in &self.edges {
            if edge.from == node_id || edge.to == node_id {
                centrality += edge.weight;
            }
        }

        centrality
    }
}