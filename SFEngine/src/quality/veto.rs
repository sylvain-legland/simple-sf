// Ref: FT-SSF-026

#[derive(Debug, Clone, PartialEq)]
pub enum VetoLevel {
    Absolute,
    Strong,
    Advisory,
}

#[derive(Debug, Clone)]
pub struct Veto {
    pub from: String,
    pub level: VetoLevel,
    pub reason: String,
    pub phase: String,
}

pub struct VetoSystem {
    pub vetoes: Vec<Veto>,
    pub overrides: Vec<String>,
}

// Hierarchy rank: lower = more authority
const HIERARCHY: &[(&str, u8)] = &[
    ("trace-lead", 1),
    ("architect", 2),
    ("lead-dev", 3),
    ("qa", 4),
    ("developer", 5),
];

fn rank_of(role: &str) -> u8 {
    HIERARCHY.iter().find(|(r, _)| *r == role).map(|(_, v)| *v).unwrap_or(99)
}

impl VetoSystem {
    pub fn new() -> Self {
        Self {
            vetoes: Vec::new(),
            overrides: Vec::new(),
        }
    }

    pub fn cast_veto(&mut self, veto: Veto) {
        self.vetoes.push(veto);
    }

    pub fn can_proceed(&self, phase: &str) -> bool {
        for (i, v) in self.vetoes.iter().enumerate() {
            if v.phase != phase {
                continue;
            }
            let overridden = self.overrides.contains(&format!("{}", i));
            match v.level {
                VetoLevel::Absolute => return false, // never overridable
                VetoLevel::Strong if !overridden => return false,
                _ => {}
            }
        }
        true
    }

    /// Override a veto by index. Only Strong and Advisory can be overridden,
    /// and the authority must outrank the veto caster.
    pub fn override_veto(&mut self, veto_idx: usize, authority: &str) {
        if let Some(veto) = self.vetoes.get(veto_idx) {
            if veto.level == VetoLevel::Absolute {
                return; // cannot override absolute
            }
            if rank_of(authority) < rank_of(&veto.from) {
                self.overrides.push(format!("{}", veto_idx));
            }
        }
    }

    pub fn active_vetoes(&self) -> Vec<&Veto> {
        self.vetoes
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.overrides.contains(&format!("{}", i)))
            .map(|(_, v)| v)
            .collect()
    }
}
