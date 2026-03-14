// Ref: FT-SSF-023

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum KanbanState {
    Backlog,
    Todo,
    InProgress,
    Review,
    Done,
    Blocked,
}

impl KanbanState {
    fn label(&self) -> &str {
        match self {
            Self::Backlog => "Backlog",
            Self::Todo => "Todo",
            Self::InProgress => "InProgress",
            Self::Review => "Review",
            Self::Done => "Done",
            Self::Blocked => "Blocked",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KanbanItem {
    pub id: String,
    pub title: String,
    pub state: KanbanState,
    pub wip_class: String,
}

pub struct KanbanBoard {
    pub items: Vec<KanbanItem>,
    pub wip_limits: HashMap<String, usize>,
}

impl KanbanBoard {
    pub fn new() -> Self {
        let mut wip_limits = HashMap::new();
        wip_limits.insert("InProgress".to_string(), 3);
        wip_limits.insert("Review".to_string(), 2);
        Self { items: Vec::new(), wip_limits }
    }

    pub fn move_item(&mut self, id: &str, to: KanbanState) -> Result<(), String> {
        let col = to.label().to_string();
        if let Some(limit) = self.wip_limits.get(&col) {
            let count = self.items.iter().filter(|i| i.state == to).count();
            if count >= *limit {
                return Err(format!("WIP limit reached for {} ({}/{})", col, count, limit));
            }
        }
        let item = self.items.iter_mut().find(|i| i.id == id)
            .ok_or_else(|| format!("Item '{}' not found", id))?;
        item.state = to;
        Ok(())
    }

    pub fn is_blocked(&self) -> bool {
        for (col, limit) in &self.wip_limits {
            let count = self.items.iter().filter(|i| i.state.label() == col).count();
            if count > *limit {
                return true;
            }
        }
        false
    }

    pub fn board_summary(&self) -> String {
        let columns = [
            KanbanState::Backlog, KanbanState::Todo, KanbanState::InProgress,
            KanbanState::Review, KanbanState::Done, KanbanState::Blocked,
        ];
        let mut out = String::new();
        for col in &columns {
            let items: Vec<&KanbanItem> = self.items.iter().filter(|i| i.state == *col).collect();
            out.push_str(&format!("[ {} ({}) ]\n", col.label(), items.len()));
            for item in &items {
                out.push_str(&format!("  - {} ({})\n", item.title, item.id));
            }
        }
        out
    }
}
