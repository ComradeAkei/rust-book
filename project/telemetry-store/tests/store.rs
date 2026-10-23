//! Общие тесты шага П15 против эталонной реализации проекта плюс точная
//! проверка «в установившемся режиме выделений ноль» — запуском примера
//! в отдельном (однопоточном) процессе, где счётчик детерминирован.
use telemetry_store as sut;

include!("../../../tests-shared/store.rs");

mod steady {
    /// Пример печатает число выделений за установившийся приём. В своём
    /// процессе счётчик видит только наши выделения, поэтому проверка
    /// точная, в отличие от общих тестов, идущих параллельно.
    #[test]
    fn p15_example_steady_state_zero_allocs() {
        let run = std::process::Command::new(env!("CARGO"))
            .args(["run", "-q", "--example", "store", "-p", "telemetry-store"])
            .output()
            .expect("запуск примера");
        let text = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success(),
            "пример упал: {text}\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let line = text.lines().find(|l| l.starts_with("установившийся приём")).unwrap_or("");
        assert!(line.contains("выделений 0,"), "ожидали ноль выделений, вывод: {line:?}");
        assert!(line.contains("занято не изменилось"), "занятая память изменилась: {line:?}");
    }
}
