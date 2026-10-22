// Задача 5.D, часть 1: окна через стандартный `Iterator` не компилируются.
//
// Если бы реализация компилировалась, два окна из двух вызовов `next` жили
// бы одновременно и делили элемент:
//
//     let mut it = WindowsMut { slice: &mut v[..], size: 2, pos: 0 };
//     let (a, b) = (it.next().unwrap(), it.next().unwrap());
//     a[1] = 1; b[0] = 2; // a[1] и b[0] — один и тот же v[1]

pub struct WindowsMut<'s, T> {
    slice: &'s mut [T],
    size: usize,
    pos: usize,
}

impl<'s, T> Iterator for WindowsMut<'s, T> {
    // Тип элемента один на весь итератор и не может зависеть от
    // заимствования `&mut self` в `next`. Остаётся `'s` — но тогда два
    // окна из двух вызовов `next` жили бы одновременно и пересекались.
    type Item = &'s mut [T];

    fn next(&mut self) -> Option<&'s mut [T]> {
        let end = self.pos.checked_add(self.size)?;
        let w = self.slice.get_mut(self.pos..end)?;
        self.pos += 1;
        Some(w) // окно заимствует `*self` только на время этого вызова
    }
}
