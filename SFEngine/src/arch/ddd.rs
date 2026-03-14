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
