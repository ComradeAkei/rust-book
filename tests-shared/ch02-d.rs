// Тесты задачи 2.D: временный `MutexGuard` и взаимная блокировка.
//
// Файл подключается внутрь `mod task_d` дважды: из `ch02.rs` (крейты
// edition 2024) и из тестов крейтов `ch02-*-2021`. Поэтому в нём только
// синтаксис, общий для обеих редакций, а ожидания выбираются по константе
// `sut::EDITION`.
//
// Каждая функция вызывается в отдельном потоке. Если поток за `TIMEOUT` не
// вернулся, а мьютекс всё ещё захвачен, функция считается зависшей; сам
// поток так и висит до конца процесса, поэтому кэш он держит через `Arc`
// и тот не освобождается, пока поток жив. Под Miri зависание — ошибка всей
// программы, так что тест с пустым кэшем там пропускается.

use super::sut::EDITION;
use super::sut::d_deadlock::{self as d, Cache};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

const NAMES: [&str; 5] = ["via_if", "via_if_let", "via_match", "via_let_else", "via_while_let"];
const FUNCS: [fn(&Cache) -> u32; 5] =
    [d::via_if, d::via_if_let, d::via_match, d::via_let_else, d::via_while_let];

/// Сколько ждать возврата: захват мьютекса занимает наносекунды, так что
/// полсекунды хватит даже на перегруженной машине CI.
const TIMEOUT: Duration = Duration::from_millis(500);

/// Чем кончился вызов.
#[derive(Debug, PartialEq)]
enum Outcome {
    Returned(u32),
    /// За `TIMEOUT` не вернулась, а мьютекс всё ещё захвачен.
    Deadlock,
}

/// Кэш с начальным значением, который переживёт зависший поток.
fn new_cache(init: Option<u32>) -> Arc<Cache> {
    Arc::new(Mutex::new(init))
}

/// Вызывает `f(cache)` в потоке с именем `name`. Паника в потоке (например,
/// `todo!()` в заготовке) передаётся дальше, в тест.
fn run(name: &str, f: fn(&Cache) -> u32, cache: &Arc<Cache>) -> Outcome {
    let (tx, rx) = mpsc::channel();
    let shared = Arc::clone(cache);
    let handle = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            let _ = tx.send(f(&shared));
        })
        .unwrap();
    match rx.recv_timeout(TIMEOUT) {
        Ok(v) => {
            handle.join().unwrap();
            Outcome::Returned(v)
        }
        Err(RecvTimeoutError::Timeout) => {
            assert!(
                matches!(cache.try_lock(), Err(TryLockError::WouldBlock)),
                "{name} не вернулась за {TIMEOUT:?}, но мьютекс свободен: \
                 это не взаимная блокировка, а бесконечный цикл"
            );
            Outcome::Deadlock
        }
        Err(RecvTimeoutError::Disconnected) => match handle.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => unreachable!("поток завершился, не отправив результат"),
        },
    }
}

/// Предсказания для редакции, в которой собран проверяемый крейт.
fn predictions() -> [Option<bool>; 5] {
    match EDITION {
        2021 => d::HANGS_2021,
        2024 => d::HANGS_2024,
        e => panic!("неизвестная редакция {e}"),
    }
}

#[test]
fn predictions_cover_task() {
    for (edition, hangs) in [(2021, d::HANGS_2021), (2024, d::HANGS_2024)] {
        for (name, h) in NAMES.iter().zip(hangs) {
            assert!(h.is_some(), "HANGS_{edition}: не заполнено предсказание для {name}");
        }
    }
    // Поведение проверяет `behavior_matches_predictions` в каждой из двух
    // редакций, а здесь — что предсказания вообще отвечают на вопросы
    // задачи. Если они сбудутся, найден и вариант, который виснет только
    // до edition 2024, и вариант, который виснет в обеих.
    let both = d::HANGS_2021.iter().zip(d::HANGS_2024);
    let pairs: Vec<_> = both.map(|(&a, b)| (a == Some(true), b == Some(true))).collect();
    assert!(
        pairs.contains(&(true, false)),
        "ни одна функция не виснет в 2021 и не возвращается в 2024: \
         разница редакций не показана"
    );
    assert!(pairs.contains(&(true, true)), "нет функции, которая виснет в обеих редакциях");
}

#[test]
#[cfg_attr(miri, ignore = "под Miri взаимная блокировка останавливает всю программу")]
fn behavior_matches_predictions() {
    let mut wrong = Vec::new();
    for ((name, f), predicted) in NAMES.iter().zip(FUNCS).zip(predictions()) {
        let cache = new_cache(None);
        let hangs = match run(name, f, &cache) {
            Outcome::Deadlock => true,
            Outcome::Returned(v) => {
                assert_eq!(v, d::FRESH, "{name}: на пустом кэше должна вернуть FRESH");
                assert_eq!(
                    *cache.lock().unwrap(),
                    Some(d::FRESH),
                    "{name}: FRESH не записано в кэш"
                );
                false
            }
        };
        if Some(hangs) != predicted {
            let did = if hangs { "зависла" } else { "вернулась" };
            let said = match predicted {
                Some(true) => "зависнет",
                Some(false) => "вернётся",
                None => "ничего",
            };
            wrong.push(format!("{name}: {did}, а предсказано «{said}»"));
        }
    }
    assert!(
        wrong.is_empty(),
        "edition {EDITION}, пустой кэш, предсказания HANGS_{EDITION} не сбылись:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn cached_value_is_returned() {
    let all = NAMES.iter().zip(FUNCS).chain([(&"fixed", d::fixed as fn(&Cache) -> u32)]);
    for (name, f) in all {
        let cache = new_cache(Some(7));
        assert_eq!(run(name, f, &cache), Outcome::Returned(7), "{name}: в кэше уже есть 7");
        assert_eq!(*cache.lock().unwrap(), Some(7), "{name}: кэш не должен меняться");
    }
}

#[test]
fn fixed_fills_empty_cache() {
    let cache = new_cache(None);
    assert_eq!(run("fixed", d::fixed, &cache), Outcome::Returned(d::FRESH));
    assert_eq!(*cache.lock().unwrap(), Some(d::FRESH), "fixed: FRESH не записано в кэш");
}

#[test]
fn fixed_under_contention() {
    // Восемь потоков одновременно заполняют пустой кэш: все должны увидеть
    // одно и то же значение, и ни один не должен зависнуть.
    let cache = new_cache(None);
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let cache = Arc::clone(&cache);
            std::thread::spawn(move || run(&format!("fixed #{i}"), d::fixed, &cache))
        })
        .collect();
    for t in threads {
        assert_eq!(t.join().unwrap(), Outcome::Returned(d::FRESH));
    }
}
