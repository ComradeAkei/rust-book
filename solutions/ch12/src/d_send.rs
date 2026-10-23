//! Задача D: одна строка в теле замыкания делает его не `Send`, и
//! компиляция ломается у `thread::spawn` в другом крейте.
//!
//! `Job` держит `Rc`, но замыкание из `make_job` захватывает не `job`, а
//! только поле `job.id` (точный захват, раздел 12.1), и поэтому `Send`.
//! Строка под `#[cfg(feature = "d-break")]` трогает `job.stats`: замыкание
//! захватывает `Rc` и перестаёт быть `Send`. Тип результата — `impl FnMut`,
//! а auto traits скрытого типа просачиваются сквозь `impl Trait`, поэтому
//! сигнатура `make_job` не меняется, и ошибка появляется там, где
//! замыкание отдают в поток. `make_job_send` обещает `Send` в сигнатуре, и
//! та же строка под `d-break-send` ломает уже её.

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

// ANCHOR: make-job
/// Задание считает свои вызовы: `id * 100 + номер вызова`.
pub fn make_job(job: Job) -> impl FnMut() -> u32 + 'static {
    let mut calls = 0;
    move || {
        calls += 1;
        #[cfg(feature = "d-break")]
        job.stats.set(job.stats.get() + 1);
        job.id * 100 + calls
    }
}

/// То же, но `Send` обещан в сигнатуре: ошибка, если обещание нарушено,
/// возникает здесь, а не у вызывающего.
pub fn make_job_send(job: Job) -> impl FnMut() -> u32 + Send + 'static {
    let mut calls = 0;
    move || {
        calls += 1;
        #[cfg(feature = "d-break-send")]
        job.stats.set(job.stats.get() + 1);
        job.id * 100 + calls
    }
}
// ANCHOR_END: make-job

// ANCHOR: future
/// Для сравнения: у `async`-блока `Send` зависит не только от захваченного,
/// но и от локальных переменных, живых в точке `.await`.
pub async fn tick() {}

pub async fn make_task(id: u32) -> u32 {
    #[cfg(feature = "d-break")]
    let _guard = Rc::new(id);
    tick().await;
    id + 1
}

/// Rc, созданный и уничтоженный внутри тела замыкания, на `Send` замыкания
/// не влияет: тип замыкания складывается только из захваченного.
pub fn make_local_rc(id: u32) -> impl FnMut() -> u32 + Send + 'static {
    move || {
        let local = Rc::new(id);
        *local + 2
    }
}
// ANCHOR_END: future
