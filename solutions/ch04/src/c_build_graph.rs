//! Задача 4.C: граф зависимостей сборки на арене с номерами.
//!
//! Задачи лежат в векторах графа, ребро — номер задачи. Ссылок между
//! узлами нет, поэтому циклы в графе не мешают проверке заимствований.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Номер задачи в арене.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u32);

/// Задачи сборки и зависимости между ними.
#[derive(Default)]
pub struct Graph {
    names: Vec<String>,
    deps: Vec<Vec<TaskId>>,
    by_name: BTreeMap<String, TaskId>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Номер задачи по имени. Задачу, которой ещё нет, добавляет.
    pub fn task(&mut self, name: &str) -> TaskId {
        // Возвращается копия номера, а не ссылка в словарь: вставка ниже
        // проверке заимствований не мешает (сравните с разделом 4.6).
        if let Some(&id) = self.by_name.get(name) {
            return id;
        }
        let id = TaskId(u32::try_from(self.names.len()).expect("больше 2^32 задач"));
        self.names.push(String::from(name));
        self.deps.push(Vec::new());
        self.by_name.insert(String::from(name), id);
        id
    }

    /// `task` нельзя начинать, пока не готова `dep`.
    pub fn depends(&mut self, task: TaskId, dep: TaskId) {
        self.deps[task.0 as usize].push(dep);
    }

    pub fn name(&self, id: TaskId) -> &str {
        &self.names[id.0 as usize]
    }

    /// Прямые зависимости задачи.
    pub fn deps(&self, id: TaskId) -> &[TaskId] {
        &self.deps[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Порядок сборки: каждая задача после всех своих зависимостей. Если
    /// есть цикл, `Err` с его задачами: каждая зависит от следующей,
    /// последняя — от первой.
    ///
    /// Обход в глубину со своим стеком вместо рекурсии: цепочка из 100 000
    /// задач переполнила бы стек потока.
    pub fn build_order(&self) -> Result<Vec<TaskId>, Vec<TaskId>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            New,
            OnStack,
            Done,
        }
        let mut mark = vec![Mark::New; self.len()];
        let mut order = Vec::with_capacity(self.len());
        // Задача и номер её следующей непросмотренной зависимости.
        let mut stack: Vec<(TaskId, usize)> = Vec::new();
        for start in 0..self.len() {
            if mark[start] != Mark::New {
                continue;
            }
            mark[start] = Mark::OnStack;
            stack.push((TaskId(start as u32), 0));
            while let Some(top) = stack.last_mut() {
                let (id, next) = *top;
                let Some(&dep) = self.deps(id).get(next) else {
                    // Все зависимости готовы: задача тоже.
                    mark[id.0 as usize] = Mark::Done;
                    order.push(id);
                    stack.pop();
                    continue;
                };
                top.1 += 1; // последнее использование top: дальше стек свободен
                match mark[dep.0 as usize] {
                    Mark::New => {
                        mark[dep.0 as usize] = Mark::OnStack;
                        stack.push((dep, 0));
                    }
                    Mark::OnStack => {
                        // dep уже на стеке: путь от неё до вершины и есть цикл.
                        let at = stack.iter().position(|&(t, _)| t == dep).unwrap();
                        return Err(stack[at..].iter().map(|&(t, _)| t).collect());
                    }
                    Mark::Done => {}
                }
            }
        }
        Ok(order)
    }
}
