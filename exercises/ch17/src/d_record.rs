//! Задача D: свой тип динамического размера. Реализуйте `Record<T>` —
//! заголовок и хвост `[T]` переменной длины в ОДНОМ выделении. `TwoBox<T>`
//! дан для сравнения. Условие — раздел 17.7.

#[repr(C)]
pub struct Record<T> {
    pub tag: u32,
    pub data: [T],
}

impl<T> Record<T> {
    // ANCHOR: skeleton
    /// Соберите `Box<Record<T>>` в одном выделении: посчитайте раскладку
    /// (`Layout::new::<u32>().extend(Layout::array::<T>(n))`), выделите
    /// память, запишите `tag`, перенесите элементы в хвост через
    /// `copy_nonoverlapping` и затем `Vec::set_len(0)`, соберите толстый
    /// указатель `slice_from_raw_parts_mut(thin as *mut T, n) as *mut
    /// Record<T>` и оберните его `Box::from_raw`. Своего `Drop` не пишите —
    /// объясните в комментарии почему.
    pub fn new(tag: u32, items: Vec<T>) -> Box<Record<T>> {
        let _ = (tag, items);
        todo!("раздел 17.7, задача D")
    }
    // ANCHOR_END: skeleton
}

/// Та же задача через два выделения — дано для сравнения.
pub struct TwoBox<T> {
    pub tag: u32,
    pub data: Box<[T]>,
}

impl<T> TwoBox<T> {
    pub fn new(tag: u32, items: Vec<T>) -> Self {
        TwoBox { tag, data: items.into_boxed_slice() }
    }
}
