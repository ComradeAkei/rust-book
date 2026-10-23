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
pub struct Buffered<S: Sink> {
    /// `None` после `finish`: деструктору нечего делать.
    sink: Option<S>,
    buf: [u8; CAP],
    len: usize,
    /// Приёмник запаниковал посреди `send`: его состояние неизвестно, и
    /// трогать его в деструкторе нельзя.
    poisoned: bool,
}

impl<S: Sink> Buffered<S> {
    pub fn new(sink: S) -> Self {
        Buffered { sink: Some(sink), buf: [0; CAP], len: 0, poisoned: false }
    }

    /// Положить данные в буфер; когда он полон, сбросить в приёмник.
    pub fn write(&mut self, mut data: &[u8]) -> Result<(), SinkError> {
        while !data.is_empty() {
            if self.len == CAP {
                self.flush()?;
            }
            let n = (CAP - self.len).min(data.len());
            self.buf[self.len..self.len + n].copy_from_slice(&data[..n]);
            self.len += n;
            data = &data[n..];
        }
        Ok(())
    }

    /// Отправить всё, что в буфере. При ошибке отправленная часть
    /// удаляется из буфера, остальное остаётся.
    pub fn flush(&mut self) -> Result<(), SinkError> {
        let Some(sink) = self.sink.as_mut() else { return Ok(()) };
        let mut sent = 0;
        let result = loop {
            if sent == self.len {
                break Ok(());
            }
            self.poisoned = true;
            let r = sink.send(&self.buf[sent..self.len]);
            self.poisoned = false;
            match r {
                // Приёмник нарушил контракт: ноль байт зациклил бы сброс,
                // а лишние вывели бы за границу буфера. Данные не теряем.
                Ok(0) => break Err(SinkError::Full),
                Ok(n) => sent += n.min(self.len - sent),
                Err(e) => break Err(e),
            }
        };
        self.buf.copy_within(sent..self.len, 0);
        self.len -= sent;
        result
    }

    /// Сбросить буфер и вернуть приёмник. Единственный способ узнать об
    /// ошибке последнего сброса.
    pub fn finish(mut self) -> Result<S, Unsent<S>> {
        let r = self.flush();
        let sink = self.sink.take().expect("sink есть до finish");
        match r {
            Ok(()) => Ok(sink),
            Err(error) => Err(Unsent { sink, pending: self.len, error }),
        }
    }
}

impl<S: Sink> Drop for Buffered<S> {
    fn drop(&mut self) {
        // Во время раскрутки и после паники приёмника не отправляем ничего:
        // вторая паника из send означала бы abort. Ошибку сброса здесь
        // некому вернуть, поэтому она теряется, как в std::io::BufWriter;
        // кому она важна, вызывает finish.
        if !std::thread::panicking() && !self.poisoned {
            let _ = self.flush();
        }
    }
}
// ANCHOR_END: buffered
