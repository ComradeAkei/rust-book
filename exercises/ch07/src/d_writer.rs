//! Задача D: буферизованная запись, которая не превращает ошибку или
//! панику в abort.

// ANCHOR: sink
/// Ошибка приёмника данных.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SinkError {
    Disconnected,
    Full,
}

/// Приёмник: UART, сокет, файл. `send` принимает от 1 до `data.len()`
/// байт и сообщает, сколько принял, или возвращает ошибку.
pub trait Sink {
    fn send(&mut self, data: &[u8]) -> Result<usize, SinkError>;
}

/// Ёмкость буфера.
pub const CAP: usize = 64;

/// Результат `finish`, когда сбросить буфер не удалось: приёмник
/// возвращается вызывающему вместе с числом неотправленных байт.
#[derive(Debug)]
pub struct Unsent<S> {
    pub sink: S,
    pub pending: usize,
    pub error: SinkError,
}
// ANCHOR_END: sink

// ANCHOR: buffered
/// ЗАГОТОВКА: поля и деструктор спроектируйте сами. Буфер — массив `CAP`
/// байт внутри структуры, без выделения памяти.
pub struct Buffered<S: Sink> {
    sink: S,
}

impl<S: Sink> Buffered<S> {
    pub fn new(sink: S) -> Self {
        todo!()
    }

    /// Положить данные в буфер; когда он полон, сбросить в приёмник.
    pub fn write(&mut self, data: &[u8]) -> Result<(), SinkError> {
        todo!()
    }

    /// Отправить всё, что в буфере. При ошибке отправленная часть
    /// удаляется из буфера, остальное остаётся.
    pub fn flush(&mut self) -> Result<(), SinkError> {
        todo!()
    }

    /// Сбросить буфер и вернуть приёмник или `Unsent` с ошибкой.
    pub fn finish(self) -> Result<S, Unsent<S>> {
        todo!()
    }
}
// ANCHOR_END: buffered
