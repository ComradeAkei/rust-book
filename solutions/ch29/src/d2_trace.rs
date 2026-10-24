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

// ANCHOR: line
/// Строка трассы: процессор, время (если есть) и текст справа от `|`.
fn split_line(line: &str) -> Option<(u32, Option<f64>, &str)> {
    let (cpu, rest) = line.trim_start().split_once(')')?;
    let cpu = cpu.trim().parse().ok()?;
    let (left, code) = rest.split_once('|')?;
    let time = left
        .trim()
        .trim_start_matches(['+', '!', '#', '*', '@', '$'])
        .trim()
        .strip_suffix("us")
        .and_then(|t| t.trim().parse().ok());
    Some((cpu, time, code))
}
// ANCHOR_END: line

// ANCHOR: calls
/// Все завершённые вызовы трассы в порядке завершения. Вызовы, начало
/// которых не попало в трассу, берут имя из комментария funcgraph-tail;
/// строки прерываний (`==========>`) и переключений задач пропускаются.
pub fn calls(trace: &str) -> Vec<Call> {
    let mut out = Vec::new();
    // Открытые вызовы — свои на каждом процессоре.
    let mut open: Vec<(u32, String, usize)> = Vec::new();
    for line in trace.lines().filter(|l| !l.starts_with('#')) {
        let Some((cpu, time, code)) = split_line(line) else { continue };
        // Два пробела после `|`, потом по два на уровень вложенности.
        let text = code.trim_start();
        let depth = (code.len() - text.len()).saturating_sub(1) / 2;
        let text = text.trim_end();
        if let Some(name) = text.strip_suffix("() {") {
            open.push((cpu, name.to_string(), depth));
        } else if let Some(name) = text.strip_suffix("();") {
            if let Some(micros) = time {
                out.push(Call { cpu, function: name.to_string(), depth, micros });
            }
        } else if text.starts_with('}') {
            let tail = text.strip_prefix("} /* ").and_then(|t| t.strip_suffix(" */"));
            let pos = open.iter().rposition(|(c, _, d)| *c == cpu && *d == depth);
            let function = match (pos, tail) {
                (Some(i), _) => open.remove(i).1,
                (None, Some(name)) => name.to_string(),
                (None, None) => continue,
            };
            if let Some(micros) = time {
                out.push(Call { cpu, function, depth, micros });
            }
        }
    }
    out
}
// ANCHOR_END: calls

/// Длительности вызовов функции `name` (имя без модуля в скобках), мкс.
pub fn durations(trace: &str, name: &str) -> Vec<f64> {
    calls(trace)
        .into_iter()
        .filter(|c| c.function.split(' ').next() == Some(name))
        .map(|c| c.micros)
        .collect()
}

/// Медиана; `None` для пустого набора.
pub fn median(values: &[f64]) -> Option<f64> {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(v[n / 2]),
        _ => Some((v[n / 2 - 1] + v[n / 2]) / 2.0),
    }
}

// ANCHOR: objdump
/// Число инструкций функции `symbol` в выводе `objdump -d`: строки от
/// заголовка `<symbol>:` до пустой строки, кроме строк перемещений
/// (`objdump -r`). `None` — функции в выводе нет.
pub fn instructions(objdump: &str, symbol: &str) -> Option<usize> {
    let header = format!("<{symbol}>:");
    let mut lines = objdump.lines().skip_while(|l| !l.ends_with(&header));
    lines.next()?;
    Some(
        lines
            .take_while(|l| !l.trim().is_empty())
            .filter(|l| {
                // `     1c:  callq ...` — инструкция; `\t\t000000000000001d:  R_X86_64_...` — перемещение.
                let t = l.trim_start();
                t.split_once(':').is_some_and(|(addr, rest)| {
                    addr.bytes().all(|b| b.is_ascii_hexdigit())
                        && !rest.trim_start().starts_with("R_")
                })
            })
            .count(),
    )
}
// ANCHOR_END: objdump

/// Вызовы (`call`) внутри функции `symbol`: куда они ведут — по
/// перемещениям `R_X86_64_PLT32`, которые печатает `objdump -d -r`.
pub fn callees(objdump: &str, symbol: &str) -> Vec<String> {
    let header = format!("<{symbol}>:");
    let mut lines = objdump.lines().skip_while(|l| !l.ends_with(&header));
    if lines.next().is_none() {
        return Vec::new();
    }
    let body: Vec<&str> = lines.take_while(|l| !l.trim().is_empty()).collect();
    body.windows(2)
        .filter(|w| w[0].contains("call"))
        .filter_map(|w| {
            let reloc = w[1].split_once("R_X86_64_PLT32")?.1.trim();
            Some(reloc.strip_suffix("-0x4").unwrap_or(reloc).to_string())
        })
        .collect()
}
