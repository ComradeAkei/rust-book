//! Задача B: иерархия ошибок загрузки конфигурации станции поверх лексера
//! из задачи 5.B (берётся из `solutions/ch05`).
//!
//! ЗАГОТОВКА. Типы `StationConfig` и `Sensor` и константы даны, `ConfigError`
//! спроектируйте сами: тесты требуют, чтобы он реализовал
//! `Error + Send + Sync + 'static` и занимал не больше 40 байт. Отчёт
//! `report` печатает сообщение и все причины:
//!
//! ```text
//! ошибка: <сообщение ошибки>
//!   причина: <сообщение source()>
//!   причина: <сообщение source() у причины>
//! ```
//!
//! Сообщения звеньев цепочки (условие в разделе 7.9):
//!
//! | Звено | Сообщение | `source` |
//! |---|---|---|
//! | файл не прочитан | `не удалось прочитать <путь>` | `io::Error` |
//! | ошибка разбора файла | `<путь>: конфигурация отвергнута` | ошибка `parse` |
//! | ошибка лексера | `синтаксическая ошибка` | позиция |
//! | позиция | `строка <N>, столбец <M>: <что>` | нет |
//! | смысловая ошибка | `ошибка в строке <N>` | что именно |
//! | что именно | см. таблицу в разделе 7.9 | `ParseIntError` у нечисла |

use ch05_solutions::b_lexer::{LexError, LexErrorKind, Lexer, Token};
use std::error::Error;
use std::fmt;
use std::io;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

/// Скорости UART, которые поддерживает станция.
pub const BAUDS: [u32; 5] = [9600, 19200, 38400, 57600, 115200];
/// Скорость, если в секции `[uart]` нет ключа `baud` или нет самой секции.
pub const DEFAULT_BAUD: u32 = 115200;
/// Допустимый период опроса датчика, мс.
pub const PERIOD_MS: core::ops::RangeInclusive<u32> = 10..=60_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StationConfig {
    pub baud: u32,
    /// Датчики в порядке появления в файле.
    pub sensors: Vec<Sensor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sensor {
    pub id: u16,
    pub period_ms: u32,
}

/// Замените своей иерархией ошибок.
#[derive(Debug)]
pub struct ConfigError;

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for ConfigError {}

pub fn parse(src: &str) -> Result<StationConfig, ConfigError> {
    todo!()
}

pub fn load(path: &Path) -> Result<StationConfig, ConfigError> {
    todo!()
}

/// Отчёт: сообщение и все причины по одной на строку.
pub fn report(e: &(dyn Error + 'static)) -> String {
    todo!()
}
