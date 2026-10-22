//! Задача A: какие из шести цепочек `?` скомпилируются и каких реализаций
//! трейтов им не хватает.

// ANCHOR: verdict
/// Ответ для одного фрагмента из `code/ch07/fragments`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Фрагмент компилируется как есть.
    Compiles,
    /// Не компилируется. Реализации, перечисленные Rust-кодом, дописанные
    /// в конец фрагмента, это исправляют, и ни одна из них не лишняя.
    Missing(&'static [&'static str]),
    /// Не компилируется, и никакая реализация трейта не поможет: нужен
    /// другой код.
    Never,
}
// ANCHOR_END: verdict

// ANCHOR: answers
pub fn answers() -> [Verdict; 6] {
    [
        // a1: io::Error, &str и ParseIntError — у всех есть From в Box<dyn Error>.
        Verdict::Compiles,
        // a2: для ParseIntError преобразования нет.
        Verdict::Missing(&["impl From<std::num::ParseIntError> for ConfigError {
            fn from(_: std::num::ParseIntError) -> Self { ConfigError::Range(0) }
        }"]),
        // a3: `?` на Option в функции, которая возвращает Result.
        Verdict::Never,
        // a4: From не транзитивен: io::Error → ConfigError → AppError сам
        // не сложится.
        Verdict::Missing(&["impl From<io::Error> for AppError {
            fn from(e: io::Error) -> Self { AppError::Config(ConfigError(e)) }
        }"]),
        // a5: TryFromSliceError и String превращаются в Box<dyn Error + Send + Sync>.
        Verdict::Compiles,
        // a6: не хватает не From, а самого Error.
        Verdict::Missing(&["impl Error for SensorFault {}"]),
    ]
}
// ANCHOR_END: answers
