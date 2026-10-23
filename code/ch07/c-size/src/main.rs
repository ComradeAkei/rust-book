//! Статистика по отсчётам из аргументов командной строки:
//! `ch07-c-size 10 20 30` печатает минимум, максимум и среднее.
//! Программа нарочно использует то, что даёт посадочные площадки:
//! владеющие значения с деструкторами, `?`, `unwrap` и индексацию.

use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum StatsError {
    Empty,
    Bad { arg: String, source: ParseIntError },
}

impl fmt::Display for StatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatsError::Empty => f.write_str("нет отсчётов"),
            StatsError::Bad { arg, source } => write!(f, "{arg}: {source}"),
        }
    }
}

struct Stats {
    samples: Vec<i32>,
    names: Vec<String>,
}

fn parse(args: impl Iterator<Item = String>) -> Result<Stats, StatsError> {
    let mut s = Stats { samples: Vec::new(), names: Vec::new() };
    for arg in args {
        let v = arg.parse().map_err(|source| StatsError::Bad { arg: arg.clone(), source })?;
        s.samples.push(v);
        s.names.push(arg);
    }
    if s.samples.is_empty() {
        return Err(StatsError::Empty);
    }
    Ok(s)
}

fn main() {
    match parse(std::env::args().skip(1)) {
        Ok(mut s) => {
            s.samples.sort_unstable();
            let mid = s.samples[s.samples.len() / 2];
            let sum: i64 = s.samples.iter().map(|&v| v as i64).sum();
            println!(
                "мин {} макс {} медиана {} среднее {} ({} отсчётов, первый «{}»)",
                s.samples[0],
                s.samples.last().unwrap(),
                mid,
                sum / s.samples.len() as i64,
                s.samples.len(),
                s.names[0]
            );
        }
        Err(e) => {
            eprintln!("ошибка: {e}");
            std::process::exit(2);
        }
    }
}
