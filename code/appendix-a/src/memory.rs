//! Раздел A.3: память и указатели.

// ANCHOR: mem_fns
/// `memcpy(dst, src, 4)`: длины сверяются, при расхождении будет паника.
pub fn copy_header(dst: &mut [u8; 4], src: &[u8]) {
    dst.copy_from_slice(&src[..4]);
}

/// `memmove(buf, buf + n, len - n)`: области могут перекрываться.
pub fn drop_front(buf: &mut [u8], n: usize) {
    buf.copy_within(n.., 0);
}

/// `memset(buf, 0, len)`.
pub fn wipe(buf: &mut [u8]) {
    buf.fill(0);
}
// ANCHOR_END: mem_fns

// ANCHOR: container_of
/// Узел интрузивного списка, как `struct list_head` в ядре Linux.
#[repr(C)]
pub struct ListNode {
    pub next: *mut ListNode,
}

/// Задача, в которую узел встроен полем.
#[repr(C)]
pub struct Task {
    pub id: u32,
    pub node: ListNode,
}

/// `&task->node`: указатель на поле, который кладут в список.
///
/// # Safety
///
/// `task` указывает на живой `Task`.
pub unsafe fn node_of(task: *mut Task) -> *mut ListNode {
    // SAFETY: `task` указывает на живой объект, `&raw mut` не создаёт
    // ссылку и сохраняет права исходного указателя на весь `Task`.
    unsafe { &raw mut (*task).node }
}

/// `container_of(node, struct task, node)`: от поля к объекту.
///
/// # Safety
///
/// `node` получен из `node_of` для живого `Task`. Указатель на поле,
/// взятый иначе (через `&mut task.node` или `&raw mut task.node`), может не
/// иметь прав на остальные поля объекта.
pub unsafe fn task_of(node: *mut ListNode) -> *mut Task {
    let offset = core::mem::offset_of!(Task, node);
    // SAFETY: по контракту `node` лежит внутри `Task` по смещению `offset`
    // и несёт права на весь объект, значит, результат указывает на его начало.
    unsafe { node.byte_sub(offset).cast::<Task>() }
}
// ANCHOR_END: container_of

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_functions() {
        let mut hdr = [0u8; 4];
        copy_header(&mut hdr, &[1, 2, 3, 4, 5]);
        assert_eq!(hdr, [1, 2, 3, 4]);

        let mut buf = [1, 2, 3, 4, 5];
        drop_front(&mut buf, 2);
        assert_eq!(buf, [3, 4, 5, 4, 5]);

        wipe(&mut buf);
        assert_eq!(buf, [0; 5]);
    }

    #[test]
    #[should_panic(expected = "range end index 4 out of range for slice of length 3")]
    fn memcpy_checks_bounds() {
        let mut hdr = [0u8; 4];
        copy_header(&mut hdr, &[1, 2, 3]);
    }

    #[test]
    #[should_panic(
        expected = "source slice length (3) does not match destination slice length (4)"
    )]
    fn copy_from_slice_checks_lengths() {
        let mut dst = [0u8; 4];
        dst.copy_from_slice(&[1, 2, 3]);
    }

    fn new_task() -> Task {
        Task { id: 42, node: ListNode { next: core::ptr::null_mut() } }
    }

    #[test]
    fn container_of_round_trip() {
        let mut task = new_task();
        let p = &raw mut task;
        // SAFETY: `task` жив до конца функции, `node` получен из `node_of`.
        let back = unsafe { &*task_of(node_of(p)) };
        assert_eq!(back.id, 42);
    }

    // ANCHOR: ub_container_of
    /// Указатель на поле взят прямо от переменной. Miri в модели по
    /// умолчанию (Stacked Borrows) сообщает о UB при чтении `id`:
    /// `cargo +nightly miri test -p appendix-a -- --ignored ub_container_of`.
    #[test]
    #[ignore = "UB: запускать только под Miri"]
    fn ub_container_of_from_field() {
        let mut task = new_task();
        let node = &raw mut task.node;
        let back = unsafe { &*task_of(node) };
        assert_eq!(back.id, 42);
    }
    // ANCHOR_END: ub_container_of

    #[test]
    fn box_is_freed_at_scope_end() {
        use std::cell::Cell;
        struct Noisy<'a>(&'a Cell<u32>);
        impl Drop for Noisy<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let freed = Cell::new(0);
        {
            let _b = Box::new(Noisy(&freed)); // malloc
            assert_eq!(freed.get(), 0);
        } // free: Drop для Noisy, затем освобождение памяти Box
        assert_eq!(freed.get(), 1);
    }
}
