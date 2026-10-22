//! Задача 4.C, для сравнения: рекурсивный обход в глубину. Порядок тот же,
//! что у `Graph::build_order`, но каждый уровень рекурсии — кадр стека, и
//! на цепочке из 100 000 задач стек тестового потока (2 МиБ) кончается.
//! Тест запускает его в потоке с увеличенным стеком.

use crate::c_build_graph::{Graph, TaskId};
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    New,
    OnStack,
    Done,
}

// ANCHOR: visit
/// Обойти зависимости `t`, затем добавить `t` в порядок. При цикле —
/// `Err` с задачей, на которой он замкнулся.
fn visit(g: &Graph, t: TaskId, mark: &mut [Mark], order: &mut Vec<TaskId>) -> Result<(), TaskId> {
    mark[t.0 as usize] = Mark::OnStack;
    for &d in g.deps(t) {
        match mark[d.0 as usize] {
            Mark::New => visit(g, d, mark, order)?,
            Mark::OnStack => return Err(d),
            Mark::Done => {}
        }
    }
    mark[t.0 as usize] = Mark::Done;
    order.push(t);
    Ok(())
}
// ANCHOR_END: visit

/// Порядок сборки рекурсивным обходом; при цикле — задача, на которой он
/// замкнулся (сам цикл этот вариант не восстанавливает).
pub fn build_order_recursive(g: &Graph) -> Result<Vec<TaskId>, TaskId> {
    let mut mark = vec![Mark::New; g.len()];
    let mut order = Vec::with_capacity(g.len());
    for t in 0..g.len() {
        if mark[t] == Mark::New {
            visit(g, TaskId(t as u32), &mut mark, &mut order)?;
        }
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    #[cfg_attr(miri, ignore = "100 000 уровней рекурсии под Miri слишком долго")]
    fn deep_recursion_needs_big_stack() {
        let run = || {
            let mut g = Graph::new();
            let ids: Vec<TaskId> = (0..100_000).map(|i| g.task(&format!("t{i}"))).collect();
            for w in ids.windows(2) {
                g.depends(w[0], w[1]);
            }
            assert_eq!(build_order_recursive(&g).unwrap(), g.build_order().unwrap());
        };
        // 100 000 кадров по 192 байта (отладочная сборка, x86-64) — 19,2 МБ.
        std::thread::Builder::new().stack_size(64 << 20).spawn(run).unwrap().join().unwrap();
    }
}
