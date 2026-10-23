//! Задача D: одна строка в теле замыкания делает его не `Send`, и
//! компиляция ломается у `thread::spawn` в другом крейте.
//!
//! `Job` и `run_in_thread` даны. Напишите тела `make_job`, `make_job_send`,
//! `make_task` и `make_local_rc` так, чтобы без возможностей задачи всё
//! компилировалось и тесты проходили, а затем добавьте по одной строке:
//!
//! - под `#[cfg(feature = "d-break")]` в `make_job` — строку, которая
//!   увеличивает `job.stats`, а в `make_task` — строку, после которой
//!   future перестаёт быть `Send`;
//! - под `#[cfg(feature = "d-break-send")]` в `make_job_send` — ту же
//!   строку, что в `make_job`.
//!
//! Тест собирает пробный крейт, который отдаёт `make_job` в
//! `run_in_thread`, и ждёт, что с `d-break` ошибка будет в пробном крейте,
//! а с `d-break-send` — в этом файле. Почему так, объясните в комментарии.

use std::cell::Cell;
use std::rc::Rc;

// ANCHOR: job
pub struct Job {
    pub id: u32,
    /// Статистика для журнала: общая с другими заданиями этого потока.
    pub stats: Rc<Cell<u32>>,
}

/// Запустить задание три раза в другом потоке и вернуть последний
/// результат. Так выглядит место, далёкое от `make_job`.
pub fn run_in_thread<F: FnMut() -> u32 + Send + 'static>(mut f: F) -> u32 {
    std::thread::spawn(move || {
        f();
        f();
        f()
    })
    .join()
    .unwrap()
}
// ANCHOR_END: job

/// Задание считает свои вызовы: `id * 100 + номер вызова`, начиная с 1.
pub fn make_job(job: Job) -> impl FnMut() -> u32 + 'static {
    move || todo!()
}

/// То же, но `Send` обещан в сигнатуре.
pub fn make_job_send(job: Job) -> impl FnMut() -> u32 + Send + 'static {
    move || todo!()
}

pub async fn tick() {}

/// Ждёт `tick()` и возвращает `id + 1`.
pub async fn make_task(id: u32) -> u32 {
    todo!()
}

/// Замыкание, которое создаёт `Rc::new(id)` в своём теле и возвращает
/// `*rc + 2`, оставаясь `Send`.
pub fn make_local_rc(id: u32) -> impl FnMut() -> u32 + Send + 'static {
    move || todo!()
}
