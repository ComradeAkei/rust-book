//! Задача A: таблица векторов из ELF прошивки. Скрипт компоновки
//! cortex-m-rt кладёт по адресу 0 секцию `.vector_table`: слово 0 —
//! начальное значение SP, дальше адреса обработчиков исключений и
//! прерываний. Адрес обработчика — с битом 0: процессор переходит по
//! нему, и бит 0 становится битом T регистра EPSR.

// ANCHOR: api
/// Таблица векторов: адрес секции и её байты из файла.
#[derive(Clone, Copy, Debug)]
pub struct Vectors<'a> {
    /// Адрес секции в памяти программы (`sh_addr`).
    pub addr: u32,
    bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VtError {
    /// Не ELF, не 32 бита или не little-endian.
    NotElf32,
    /// Таблица заголовков секций или строк выходит за файл.
    Truncated,
    /// Секции `.vector_table` нет.
    NoTable,
    /// Размер секции не кратен слову или меньше 16 слов.
    BadSize(u32),
    /// Начальный SP не выровнен на 8 байт.
    StackNotAligned(u32),
    /// Адрес обработчика без бита Thumb.
    EvenHandler { index: usize, addr: u32 },
}
// ANCHOR_END: api

/// Найти `.vector_table` по таблице заголовков секций и её таблице имён
/// (`e_shstrndx`). Ни одного индексирования без проверки границ: файл
/// может быть обрезан.
pub fn vector_table(elf: &[u8]) -> Result<Vectors<'_>, VtError> {
    let _ = elf;
    let _ = Vectors { addr: 0, bytes: &[] };
    todo!("задача A: vector_table")
}

impl Vectors<'_> {
    /// Число слов, начальный SP включительно.
    pub fn len(&self) -> usize {
        self.bytes.len() / 4
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Слово `i`: 0 — начальный SP, 1 — Reset, 16 и дальше — прерывания.
    pub fn get(&self, i: usize) -> Option<u32> {
        let _ = i;
        todo!("задача A: get")
    }

    /// Число прерываний после 16 слов ядра.
    pub fn interrupts(&self) -> usize {
        todo!("задача A: interrupts")
    }

    /// Начальный SP выровнен на 8 байт (AAPCS), у каждого ненулевого
    /// адреса обработчика бит 0 — бит Thumb. Нули — зарезервированные
    /// слова.
    pub fn check(&self) -> Result<(), VtError> {
        todo!("задача A: check")
    }
}
