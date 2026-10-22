//! Раздел 8.5: код, который сгенерировал `build.rs`, и данные из файлов.

/// Описание датчика. Сами значения — в `sensors.csv`.
#[derive(Debug, PartialEq, Eq)]
pub struct SensorInfo {
    pub id: u16,
    pub name: &'static str,
    pub unit: &'static str,
    pub scale: u32,
}

// ANCHOR: include
// Таблица SENSORS и функция sensor() из sensors.csv.
include!(concat!(env!("OUT_DIR"), "/sensors.rs"));

/// Для какой архитектуры работал build.rs и для какой собран крейт.
pub mod build {
    include!(concat!(env!("OUT_DIR"), "/build_info.rs"));
}
// ANCHOR_END: include

// ANCHOR: calib
/// Калибровка из двоичного файла: множитель и смещение, little-endian.
/// `include_bytes!` даёт `&[u8; N]` с выравниванием 1.
pub static CALIB: &[u8; 8] = include_bytes!("../calib.bin");

pub fn calibration() -> (u32, i32) {
    let [g0, g1, g2, g3, o0, o1, o2, o3] = *CALIB;
    (u32::from_le_bytes([g0, g1, g2, g3]), i32::from_le_bytes([o0, o1, o2, o3]))
}
// ANCHOR_END: calib

#[cfg(test)]
mod tests {
    use super::*;

    /// Таблица совпадает с CSV, разобранным независимо от build.rs.
    #[test]
    fn table_matches_csv() {
        let csv = include_str!("../sensors.csv");
        let rows: Vec<Vec<&str>> = csv
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("id,") && !l.is_empty())
            .map(|l| l.split(',').collect())
            .collect();
        assert_eq!(rows.len(), SENSORS.len());
        for (row, s) in rows.iter().zip(&SENSORS) {
            assert_eq!(row[0].parse::<u16>().unwrap(), s.id);
            assert_eq!((row[1], row[2]), (s.name, s.unit));
            assert_eq!(row[3].parse::<u32>().unwrap(), s.scale);
            assert_eq!(sensor(s.id), Some(s));
        }
        assert_eq!(sensor(3), None);
        assert_eq!(sensor(7).unwrap().unit, "Pa");
    }

    #[test]
    fn calib_from_file() {
        assert_eq!(calibration(), (10_000, -100));
        assert_eq!(core::mem::align_of_val(CALIB), 1);
    }

    #[test]
    fn build_script_runs_on_host() {
        assert_eq!(build::BUILD_RS_ARCH, std::env::consts::ARCH);
        assert_eq!(build::TARGET_ARCH, std::env::consts::ARCH);
        assert_eq!(build::BUILD_RS_CFG_ARM, cfg!(target_arch = "arm"));
    }
}
