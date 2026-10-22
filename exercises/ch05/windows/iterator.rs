// Задача 5.D, часть 1: попробуйте реализовать окна через стандартный
// `Iterator` и убедитесь, что компилятор отвергает реализацию. Тест ждёт
// ошибку «lifetime may not live long enough». В комментарии объясните,
// что пошло бы не так, если бы она компилировалась. Удалите строку с
// меткой: ЗАГОТОВКА

pub struct WindowsMut<'s, T> {
    slice: &'s mut [T],
    size: usize,
    pos: usize,
}

impl<'s, T> Iterator for WindowsMut<'s, T> {
    type Item = &'s mut [T];

    fn next(&mut self) -> Option<&'s mut [T]> {
        todo!()
    }
}
