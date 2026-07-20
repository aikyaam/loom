pub struct DependencyGraph {
    pub num_nodes: usize,
    pub edges: Vec<(usize, usize, f64)>,
}

impl DependencyGraph {
    pub fn new(num_nodes: usize) -> Self {
        Self {
            num_nodes,
            edges: Vec::new(),
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize, weight: f64) {
        self.edges.push((from, to, weight));
    }

    pub fn is_dag(&self) -> bool {
        let mut in_degree = vec![0usize; self.num_nodes];
        for &(_, to, _) in &self.edges {
            if to < self.num_nodes {
                in_degree[to] += 1;
            }
        }
        let mut queue: Vec<usize> = (0..self.num_nodes).filter(|&i| in_degree[i] == 0).collect();
        let mut visited = 0;

        while let Some(u) = queue.pop() {
            visited += 1;
            for &(from, to, _) in &self.edges {
                if from == u {
                    if to < self.num_nodes {
                        in_degree[to] = in_degree[to].saturating_sub(1);
                        if in_degree[to] == 0 {
                            queue.push(to);
                        }
                    }
                }
            }
        }
        visited == self.num_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_topology() {
        let mut graph = DependencyGraph::new(3);
        graph.add_edge(0, 1, 0.8);
        graph.add_edge(1, 2, 0.5);
        assert!(graph.is_dag());
    }
}
