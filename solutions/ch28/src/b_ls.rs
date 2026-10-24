//! Задача B: `ls -la` одного каталога без `std::fs` — getdents64 через
//! `rustix::fs::RawDir`, `fstatat` без перехода по ссылкам, `readlinkat`,
//! имена владельцев из /etc/passwd и /etc/group. Вывод совпадает байт в
//! байт с `LC_ALL=C TZ=UTC ls -la --time-style='+%Y-%m-%d %H:%M'`.
//! Устройства (`b`, `c`) в задачу не входят: у них вместо размера пара
//! «старший, младший номер» со своими правилами выравнивания.

use rustix::fs::{AtFlags, CWD, FileType, Mode, OFlags, RawDir, Stat, openat, readlinkat, statat};
use std::collections::HashMap;
use std::ffi::CStr;
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::path::Path;

// ANCHOR: entries
/// Запись каталога: имя, `lstat` и цель ссылки.
struct Entry {
    name: Vec<u8>,
    stat: Stat,
    target: Option<Vec<u8>>,
}

/// Все записи каталога, включая `.` и `..`, в порядке байтов имени —
/// так сортирует `ls` с `LC_ALL=C`.
fn entries(dir: &OwnedFd) -> io::Result<Vec<Entry>> {
    let mut buf = vec![MaybeUninit::<u8>::uninit(); 8192];
    let mut raw = RawDir::new(dir, &mut buf);
    let mut names = Vec::new();
    // getdents64 заполняет буфер записями переменной длины; RawDir идёт
    // по ним и зовёт вызов снова, когда буфер кончился.
    while let Some(entry) = raw.next() {
        names.push(entry?.file_name().to_bytes().to_vec());
    }
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let stat = statat(dir, name.as_slice(), AtFlags::SYMLINK_NOFOLLOW)?;
            let target = match FileType::from_raw_mode(stat.st_mode) {
                FileType::Symlink => {
                    Some(readlinkat(dir, name.as_slice(), Vec::new())?.into_bytes())
                }
                _ => None,
            };
            Ok(Entry { name, stat, target })
        })
        .collect()
}
// ANCHOR_END: entries

// ANCHOR: mode
/// `drwxr-xr-x`: тип и девять битов прав, с s/S и t/T на месте x.
fn mode_string(mode: u32) -> String {
    let kind = match FileType::from_raw_mode(mode) {
        FileType::Directory => 'd',
        FileType::Symlink => 'l',
        FileType::Fifo => 'p',
        FileType::Socket => 's',
        FileType::CharacterDevice => 'c',
        FileType::BlockDevice => 'b',
        _ => '-',
    };
    let bit = |mask: u32, c: char| if mode & mask != 0 { c } else { '-' };
    // Бит x вместе со специальным битом: оба — строчная, только
    // специальный — прописная.
    let exec = |x: u32, special: u32, on: char| match (mode & x != 0, mode & special != 0) {
        (true, true) => on,
        (false, true) => on.to_ascii_uppercase(),
        (true, false) => 'x',
        (false, false) => '-',
    };
    [
        kind,
        bit(0o400, 'r'),
        bit(0o200, 'w'),
        exec(0o100, 0o4000, 's'),
        bit(0o040, 'r'),
        bit(0o020, 'w'),
        exec(0o010, 0o2000, 's'),
        bit(0o004, 'r'),
        bit(0o002, 'w'),
        exec(0o001, 0o1000, 't'),
    ]
    .iter()
    .collect()
}
// ANCHOR_END: mode

// ANCHOR: names
/// Файл целиком через open и read — без `std::fs`.
fn read_all(path: &CStr) -> io::Result<Vec<u8>> {
    let fd = openat(CWD, path, OFlags::RDONLY | OFlags::CLOEXEC, Mode::empty())?;
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match rustix::io::read(&fd, &mut chunk) {
            Ok(0) => return Ok(data),
            Ok(n) => data.extend_from_slice(&chunk[..n]),
            Err(rustix::io::Errno::INTR) => continue,
            Err(e) => return Err(e.into()),
        }
    }
}

/// Номер → имя из файла формата /etc/passwd: `имя:x:номер:...`. Номер —
/// третье поле и в /etc/group. Первое вхождение номера побеждает, как у
/// getpwuid.
fn id_names(path: &CStr) -> HashMap<u32, String> {
    let text = read_all(path).unwrap_or_default();
    let mut names = HashMap::new();
    for line in String::from_utf8_lossy(&text).lines() {
        let mut fields = line.split(':');
        if let (Some(name), Some(_), Some(id)) = (fields.next(), fields.next(), fields.next())
            && let Ok(id) = id.parse()
        {
            names.entry(id).or_insert_with(|| name.to_owned());
        }
    }
    names
}
// ANCHOR_END: names

// ANCHOR: time
/// Секунды Unix → `ГГГГ-ММ-ДД ЧЧ:ММ` в UTC. Дни → дата — алгоритм
/// Говарда Хиннанта для пролептического григорианского календаря: эры по
/// 400 лет, год с марта, чтобы 29 февраля было последним днём года.
fn utc_minutes(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}
// ANCHOR_END: time

// ANCHOR: list
/// Вывод `ls -la` для каталога `dir`.
pub fn list(dir: &Path) -> io::Result<String> {
    let fd = openat(CWD, dir, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty())?;
    let entries = entries(&fd)?;
    let users = id_names(c"/etc/passwd");
    let groups = id_names(c"/etc/group");
    let name_of = |names: &HashMap<u32, String>, id: u32| {
        names.get(&id).cloned().unwrap_or_else(|| id.to_string())
    };

    struct Row {
        mode: String,
        links: String,
        user: String,
        group: String,
        size: String,
        time: String,
        name: String,
    }
    let rows: Vec<Row> = entries
        .iter()
        .map(|e| {
            let mut name = String::from_utf8_lossy(&e.name).into_owned();
            if let Some(t) = &e.target {
                name += " -> ";
                name += &String::from_utf8_lossy(t);
            }
            Row {
                mode: mode_string(e.stat.st_mode),
                links: e.stat.st_nlink.to_string(),
                user: name_of(&users, e.stat.st_uid),
                group: name_of(&groups, e.stat.st_gid),
                size: e.stat.st_size.to_string(),
                time: utc_minutes(e.stat.st_mtime),
                name,
            }
        })
        .collect();
    // Ширина колонки — по самой длинной записи; числа — вправо, имена
    // владельцев — влево.
    let width =
        |f: fn(&Row) -> &String| rows.iter().map(|r| f(r).chars().count()).max().unwrap_or(0);
    let (wl, wu, wg, ws) =
        (width(|r| &r.links), width(|r| &r.user), width(|r| &r.group), width(|r| &r.size));

    // st_blocks — в единицах по 512 байт, ls печатает КиБ с округлением
    // вверх.
    let blocks: u64 = entries.iter().map(|e| e.stat.st_blocks as u64).sum();
    let mut out = format!("total {}\n", blocks.div_ceil(2));
    for r in &rows {
        out += &format!(
            "{} {:>wl$} {:<wu$} {:<wg$} {:>ws$} {} {}\n",
            r.mode, r.links, r.user, r.group, r.size, r.time, r.name
        );
    }
    Ok(out)
}
// ANCHOR_END: list

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_dates() {
        assert_eq!(mode_string(0o040755), "drwxr-xr-x");
        assert_eq!(mode_string(0o104755), "-rwsr-xr-x");
        assert_eq!(mode_string(0o102644), "-rw-r-Sr--");
        assert_eq!(mode_string(0o041777), "drwxrwxrwt");
        assert_eq!(mode_string(0o041776), "drwxrwxrwT");
        assert_eq!(mode_string(0o120777), "lrwxrwxrwx");
        assert_eq!(mode_string(0o010600), "prw-------");
        assert_eq!(utc_minutes(0), "1970-01-01 00:00");
        assert_eq!(utc_minutes(951_782_400), "2000-02-29 00:00");
        assert_eq!(utc_minutes(1_709_213_820), "2024-02-29 13:37");
        assert_eq!(utc_minutes(-1), "1969-12-31 23:59");
    }
}
