//! Задача 29.D2: разбор трассы function_graph и вывода `objdump -d`.
//!
//! Трассировщик function_graph печатает вход в функцию строкой
//! `имя() {`, выход — `}` со временем вызова слева, а вызов без вложенных
//! — одной строкой `имя();` со временем:
//!
//! ```text
//!  1)               |  vfs_read() {
//!  1)   0.311 us    |    rw_verify_area();
//!  1) + 12.904 us   |  } /* vfs_read */
//! ```
//!
//! Слева — номер процессора и время в микросекундах; знак перед временем
//! отмечает долгие вызовы (`+` — больше 10 мкс, `!` — больше 100 и так
//! далее). Комментарий после `}` есть с опцией funcgraph-tail.

/// Завершённый вызов функции.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub cpu: u32,
    /// Имя, как его печатает ядро: `vfs_read` или `notes_read [ch29_notes_c]`.
    pub function: String,
    /// Глубина вложенности: 0 — функция из set_graph_function.
    pub depth: usize,
    /// Длительность, мкс.
    pub micros: f64,
}

/// Все завершённые вызовы трассы в порядке завершения. Вызовы, начало
/// которых не попало в трассу, берут имя из комментария funcgraph-tail;
/// строки прерываний (`==========>`) и переключений задач пропускаются.
pub fn calls(trace: &str) -> Vec<Call> {
    let _ = trace;
    todo!("29.D2: разбор трассы function_graph")
}

/// Длительности вызовов функции `name` (имя без модуля в скобках), мкс.
pub fn durations(trace: &str, name: &str) -> Vec<f64> {
    let _ = (trace, name);
    todo!("29.D2: длительности вызовов по имени")
}

/// Медиана; `None` для пустого набора.
pub fn median(values: &[f64]) -> Option<f64> {
    let _ = values;
    todo!("29.D2: медиана")
}

/// Число инструкций функции `symbol` в выводе `objdump -d`: строки от
/// заголовка `<symbol>:` до пустой строки, кроме строк перемещений
/// (`objdump -r`). `None` — функции в выводе нет.
pub fn instructions(objdump: &str, symbol: &str) -> Option<usize> {
    let _ = (objdump, symbol);
    todo!("29.D2: инструкции функции в выводе objdump")
}

/// Вызовы (`call`) внутри функции `symbol`: куда они ведут — по
/// перемещениям `R_X86_64_PLT32`, которые печатает `objdump -d -r`.
pub fn callees(objdump: &str, symbol: &str) -> Vec<String> {
    let _ = (objdump, symbol);
    todo!("29.D2: вызовы внутри функции")
}
