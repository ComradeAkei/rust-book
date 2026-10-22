//! Задача 4.C: граф зависимостей сборки на арене с номерами.
//!
//! Поля `Graph` выберите сами. Ограничения проверяет тест `arena_only`:
//! никакого небезопасного кода и счётчиков ссылок, узлы ссылаются друг на
//! друга номерами.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Номер задачи в арене.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u32);

/// Задачи сборки и зависимости между ними.
pub struct Graph {
    // ваши поля
}

impl Graph {
    pub fn new() -> Self {
        todo!()
    }

    /// Номер задачи по имени. Задачу, которой ещё нет, добавляет.
    pub fn task(&mut self, name: &str) -> TaskId {
        todo!()
    }

    /// `task` нельзя начинать, пока не готова `dep`.
    pub fn depends(&mut self, task: TaskId, dep: TaskId) {
        todo!()
    }

    pub fn name(&self, id: TaskId) -> &str {
        todo!()
    }

    /// Прямые зависимости задачи.
    pub fn deps(&self, id: TaskId) -> &[TaskId] {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Порядок сборки: каждая задача после всех своих зависимостей. Если
    /// есть цикл, `Err` с его задачами: каждая зависит от следующей,
    /// последняя — от первой.
    pub fn build_order(&self) -> Result<Vec<TaskId>, Vec<TaskId>> {
        todo!()
    }
}
