// Ref: FT-SSF-024
//! Domain-Driven Design — aggregates, entities, value objects, repositories

use std::collections::HashMap;

pub trait AggregateRoot {
    fn id(&self) -> &str;
    fn version(&self) -> u64;
}

pub trait ValueObject: PartialEq + Clone {}

pub trait DomainEvent {
    fn aggregate_id(&self) -> &str;
    fn event_type(&self) -> &str;
}

pub struct Entity<T> {
    pub id: String,
    pub version: u64,
    pub data: T,
}

pub struct Repository<T> {
    store: HashMap<String, Entity<T>>,
}

impl<T> Repository<T> {
    pub fn new() -> Self {
        Self { store: HashMap::new() }
    }

    pub fn save(&mut self, entity: Entity<T>) {
        self.store.insert(entity.id.clone(), entity);
    }

    pub fn find(&self, id: &str) -> Option<&Entity<T>> {
        self.store.get(id)
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.store.remove(id).is_some()
    }
}

impl<T> Default for Repository<T> {
    fn default() -> Self { Self::new() }
}

// --- Value Objects (newtypes) ---

#[derive(Clone, PartialEq, Debug)]
pub struct PhaseResult(pub String);
impl ValueObject for PhaseResult {}

#[derive(Clone, PartialEq, Debug)]
pub struct GuardScore(pub i32);
impl ValueObject for GuardScore {}

#[derive(Clone, PartialEq, Debug)]
pub struct AgentRole(pub String);
impl ValueObject for AgentRole {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_save_and_find() {
        let mut repo = Repository::<String>::new();
        repo.save(Entity { id: "e1".into(), version: 1, data: "hello".into() });
        let found = repo.find("e1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().data, "hello");
    }

    #[test]
    fn repository_delete() {
        let mut repo = Repository::<String>::new();
        repo.save(Entity { id: "e1".into(), version: 1, data: "x".into() });
        assert!(repo.delete("e1"));
        assert!(repo.find("e1").is_none());
        assert!(!repo.delete("e1"));
    }
}
