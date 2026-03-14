// Ref: FT-SSF-024
//! Service Mesh — microservice discovery and health routing

#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub name: String,
    pub url: String,
    pub health_path: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Down,
    Unknown,
}

pub struct ServiceMesh {
    services: Vec<ServiceEndpoint>,
}

impl ServiceMesh {
    pub fn new() -> Self {
        let mut mesh = Self { services: Vec::new() };
        mesh.register(ServiceEndpoint {
            name: "sf-engine".into(),
            url: "http://127.0.0.1".into(),
            health_path: "/health".into(),
            timeout_ms: 500,
        });
        mesh.register(ServiceEndpoint {
            name: "sf-server".into(),
            url: "http://localhost:8099".into(),
            health_path: "/api/health".into(),
            timeout_ms: 2000,
        });
        mesh.register(ServiceEndpoint {
            name: "sf-llm".into(),
            url: "http://localhost:8090".into(),
            health_path: "/health".into(),
            timeout_ms: 5000,
        });
        mesh
    }

    pub fn register(&mut self, endpoint: ServiceEndpoint) {
        self.services.push(endpoint);
    }

    pub fn discover(&self, name: &str) -> Option<&ServiceEndpoint> {
        self.services.iter().find(|s| s.name == name)
    }

    pub fn health_check_url(&self, name: &str) -> Option<String> {
        self.discover(name).map(|s| format!("{}{}", s.url, s.health_path))
    }

    pub fn all_services(&self) -> &[ServiceEndpoint] {
        &self.services
    }
}

impl Default for ServiceMesh {
    fn default() -> Self { Self::new() }
}
