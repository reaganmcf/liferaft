use std::{collections::HashMap, process::Child};

pub struct TestCluster {
    config: Configuration,
    nodes: HashMap<String, Child>,
}

pub struct Configuration {
    pub nodes: Vec<NodeConf>,
}

pub struct NodeConf {
    pub id: String,
    pub port: u16,
}

impl TestCluster {
    pub fn new(configuration: Configuration) -> Self {
        let mut nodes = HashMap::new();

        for conf in &configuration.nodes {
            let peers = &configuration
                .nodes
                .iter()
                .filter(|n| n.id != conf.id)
                .map(|n| format!("{}", n.port))
                .collect::<Vec<_>>();

            let child = std::process::Command::new("cargo")
                .arg("run")
                .arg("--")
                .arg("--id")
                .arg(&conf.id)
                .arg("--peers")
                .arg(peers.join(","))
                .arg("--port")
                .arg(conf.port.to_string())
                .spawn()
                .expect("Failed to spawn node");

            nodes.insert(conf.id.clone(), child);
        }
            
        Self {
            config: configuration,
            nodes: HashMap::new(),
        }
    }

    pub fn stop(&mut self) {
        for (_, child) in &mut self.nodes {
            child.kill().expect("Failed to kill node");
        }
    }
}
