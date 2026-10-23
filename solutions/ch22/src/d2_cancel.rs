//! Задача D2: отменобезопасное чтение кадров.
//!
//! Кадр — байт длины `N` и следом `N` байт. Источник отдаёт по байту за
//! опрос, как сокет из раздела 22.5. Наивное чтение держит прочитанное в
//! локальных переменных автомата, и отмена (проигранный `select` с
//! тайм-аутом) роняет их: поток кадров теряет синхронизацию. Отменобезопасное
//! чтение хранит прогресс в `FrameReader` — снаружи future, — поэтому
//! уронить future можно в любой точке `.await`, ничего не потеряв.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// ANCHOR: source
/// Источник байтов: по одному за опрос, с `Pending` перед каждым.
pub struct Source {
    data: Vec<u8>,
    pos: usize,
    ready: bool,
}

impl Source {
    pub fn new(data: &[u8]) -> Self {
        Source { data: data.to_vec(), pos: 0, ready: false }
    }

    /// Сколько байтов уже забрано из источника.
    pub fn consumed(&self) -> usize {
        self.pos
    }

    /// Следующий байт или `None` в конце. Сама эта future отменобезопасна:
    /// байт забирается только в том опросе, который его и возвращает.
    pub fn next_byte(&mut self) -> NextByte<'_> {
        NextByte { src: self }
    }
}

pub struct NextByte<'a> {
    src: &'a mut Source,
}

impl Future for NextByte<'_> {
    type Output = Option<u8>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<u8>> {
        let src = &mut *self.src;
        if !src.ready {
            src.ready = true; // байт «в пути»: придёт к следующему опросу
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        src.ready = false;
        let b = src.data.get(src.pos).copied();
        src.pos += b.is_some() as usize;
        Poll::Ready(b)
    }
}

/// Наивное чтение кадра: длина и тело живут в автомате. Отмена между
/// байтами роняет их вместе с автоматом, а источник забранного не вернёт.
pub async fn read_frame_naive(src: &mut Source) -> Option<Vec<u8>> {
    let len = src.next_byte().await? as usize;
    let mut body = Vec::with_capacity(len);
    while body.len() < len {
        body.push(src.next_byte().await?);
    }
    Some(body)
}
// ANCHOR_END: source

// ANCHOR: solution
/// Читатель кадров, переживающий отмену. Всё, что забрано из источника,
/// сразу кладётся сюда — в тот же опрос, без `.await` между «забрал» и
/// «сохранил». Новый вызов `read_frame` продолжает с того места, где
/// остановился уроненный.
#[derive(Default)]
pub struct FrameReader {
    need: Option<usize>, // длина текущего кадра, если её уже прочли
    body: Vec<u8>,       // прочитанная часть тела
}

impl FrameReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Прочитать очередной кадр; `None` — источник кончился.
    pub async fn read_frame(&mut self, src: &mut Source) -> Option<Vec<u8>> {
        loop {
            let need = match self.need {
                Some(n) => n,
                None => {
                    let n = src.next_byte().await? as usize;
                    self.need = Some(n); // сохранить до следующего .await
                    n
                }
            };
            if self.body.len() == need {
                self.need = None;
                return Some(std::mem::take(&mut self.body));
            }
            let b = src.next_byte().await?;
            self.body.push(b); // тоже до следующего .await
        }
    }
}
// ANCHOR_END: solution
