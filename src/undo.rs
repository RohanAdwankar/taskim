use crate::task::Task;

#[derive(Debug, Clone)]
pub enum Operation {
    DeleteTask {
        task: Task,
    },
    EditTask {
        task_id: String,
        old_task: Task,
        new_task: Task,
    },
    CreateTask {
        task: Task,
    },
}

#[derive(Debug, Clone)]
pub struct UndoStack {
    undo_operations: Vec<Operation>,
    redo_operations: Vec<Operation>,
    max_size: usize,
}

impl UndoStack {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_operations: Vec::new(),
            redo_operations: Vec::new(),
            max_size,
        }
    }

    pub fn push(&mut self, operation: Operation) {
        self.undo_operations.push(operation);

        // Clear redo stack when new operation is added
        self.redo_operations.clear();

        // Keep stack size under control
        if self.undo_operations.len() > self.max_size {
            self.undo_operations.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<Operation> {
        if let Some(operation) = self.undo_operations.pop() {
            self.redo_operations.push(operation.clone());
            Some(operation)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<Operation> {
        if let Some(operation) = self.redo_operations.pop() {
            self.undo_operations.push(operation.clone());
            Some(operation)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_operations.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_operations.is_empty()
    }
}
