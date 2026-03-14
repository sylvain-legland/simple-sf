// Ref: FT-SSF-027
//! Lazy-loading + access-controlled agent proxy

use std::collections::HashMap;

pub struct AgentProxy {
    pub agent_id: String,
    loaded: bool,
    cached_persona: Option<String>,
    access_roles: Vec<String>,
}

impl AgentProxy {
    pub fn new(agent_id: &str, allowed_roles: Vec<String>) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            loaded: false,
            cached_persona: None,
            access_roles: allowed_roles,
        }
    }

    pub fn check_access(&self, caller_role: &str) -> bool {
        self.access_roles.iter().any(|r| r == caller_role)
    }

    pub fn load(&mut self) {
        if !self.loaded {
            self.cached_persona = Some(format!("Persona for agent {}", self.agent_id));
            self.loaded = true;
        }
    }

    pub fn is_loaded(&self) -> bool { self.loaded }

    pub fn get_persona(&mut self) -> &str {
        self.load();
        self.cached_persona.as_deref().unwrap_or("unknown")
    }
}

pub struct ProxyPool {
    proxies: HashMap<String, AgentProxy>,
}

impl ProxyPool {
    pub fn new() -> Self { Self { proxies: HashMap::new() } }

    pub fn register(&mut self, proxy: AgentProxy) {
        self.proxies.insert(proxy.agent_id.clone(), proxy);
    }

    pub fn get(&mut self, agent_id: &str, caller_role: &str) -> Result<&mut AgentProxy, String> {
        let proxy = self.proxies.get_mut(agent_id)
            .ok_or_else(|| format!("Agent '{agent_id}' not registered"))?;
        if !proxy.check_access(caller_role) {
            return Err(format!("Role '{caller_role}' cannot access agent '{agent_id}'"));
        }
        proxy.load();
        Ok(proxy)
    }

    pub fn preload_all(&mut self) {
        for proxy in self.proxies.values_mut() {
            proxy.load();
        }
    }
}
