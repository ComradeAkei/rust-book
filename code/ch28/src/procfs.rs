//! Раздел 28.7: /proc и /sys. Ядро отвечает на вопросы о процессе и о
//! машине текстом, который генерирует в момент чтения; разбор этого текста
//! — обычная работа со строками, и заимствования делают его без копий.

use std::ffi::CStr;
use std::io;

// ANCHOR: region
/// Строка /proc/PID/maps: одно отображение адресного пространства.
///
/// ```text
/// 5645fb15c000-5645fb162000 r-xp 00002000 fe:00 210247     /usr/bin/head
/// 7ffcde344000-7ffcde368000 rw-p 00000000 00:00 0          [stack]
/// 55d2dc98b000-55d2dc99b000 rw-p 00000000 00:00 0
/// ```
///
/// Путь заимствуется из строки: разбор ничего не копирует.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Region<'a> {
    pub start: usize,
    pub end: usize,
    /// `r`, `w`, `x` и `p` (частное) или `s` (общее), прочерк — нет права.
    pub perms: [u8; 4],
    /// Смещение в файле, с которого отображение начинается.
    pub offset: u64,
    pub inode: u64,
    /// Файл, псевдоимя вроде `[heap]` или `[anon:имя]`, либо `None` у
    /// безымянной анонимной памяти.
    pub path: Option<&'a str>,
}

impl Region<'_> {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, addr: usize) -> bool {
        (self.start..self.end).contains(&addr)
    }
}

/// Разбор одной строки. Первые пять полей разделены одним пробелом, потом
/// ядро добивает пробелами до колонки пути. У безымянной памяти строка
/// кончается пробелом после inode, а путь сам может содержать пробелы:
/// `/tmp/a b (deleted)`. Поэтому `splitn`, а не `split_whitespace`.
pub fn parse_region(line: &str) -> Option<Region<'_>> {
    let mut fields = line.splitn(6, ' ');
    let (start, end) = fields.next()?.split_once('-')?;
    let perms = fields.next()?.as_bytes().try_into().ok()?;
    let offset = u64::from_str_radix(fields.next()?, 16).ok()?;
    let _device = fields.next()?;
    let inode = fields.next()?.parse().ok()?;
    let path = fields.next().map(str::trim_start).filter(|p| !p.is_empty());
    Some(Region {
        start: usize::from_str_radix(start, 16).ok()?,
        end: usize::from_str_radix(end, 16).ok()?,
        perms,
        offset,
        inode,
        path,
    })
}

/// Все отображения по порядку адресов. Строку, которую не удалось
/// разобрать, итератор пропускает.
pub fn regions(maps: &str) -> impl Iterator<Item = Region<'_>> {
    maps.lines().filter_map(parse_region)
}

/// Отображение, которому принадлежит адрес.
pub fn region_of(maps: &str, addr: usize) -> Option<Region<'_>> {
    regions(maps).find(|r| r.contains(addr))
}
// ANCHOR_END: region

// ANCHOR: name
/// Дать имя анонимной памяти: в maps она появится как `[anon:имя]`.
/// `prctl(PR_SET_VMA, PR_SET_VMA_ANON_NAME)`, ядро 5.17 и новее со
/// сборкой CONFIG_ANON_VMA_NAME; иначе — EINVAL.
pub fn name_anonymous(mem: &[u8], name: &CStr) -> io::Result<()> {
    Ok(rustix::process::set_virtual_memory_region_name(mem, Some(name))?)
}
// ANCHOR_END: name

// ANCHOR: status
/// Поле /proc/PID/status: `VmRSS:\t    1948 kB` → `"1948 kB"`.
pub fn status_field<'a>(status: &'a str, key: &str) -> Option<&'a str> {
    status.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix(':')).map(str::trim)
}

/// Поле status в килобайтах: `VmRSS`, `VmHWM`, `VmSize`.
pub fn status_kib(status: &str, key: &str) -> Option<u64> {
    status_field(status, key)?.strip_suffix(" kB")?.trim().parse().ok()
}

/// Сколько памяти процесса сейчас в RAM, КиБ.
pub fn rss_kib() -> io::Result<u64> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    status_kib(&status, "VmRSS").ok_or_else(|| io::Error::other("нет VmRSS"))
}
// ANCHOR_END: status

// ANCHOR: cpus
/// Список процессоров в формате ядра: `0-3,8,10-11`. Формат один у
/// /sys/devices/system/cpu/online, у `Cpus_allowed_list` в status и у
/// cpuset в cgroup.
pub fn parse_cpu_list(s: &str) -> Option<Vec<u32>> {
    let mut cpus = Vec::new();
    for part in s.trim().split(',').filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => cpus.extend(a.parse::<u32>().ok()?..=b.parse().ok()?),
            None => cpus.push(part.parse().ok()?),
        }
    }
    Some(cpus)
}
// ANCHOR_END: cpus

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::Mapped;

    #[test]
    fn parses_file_anonymous_and_named_lines() {
        let file = "5645fb15c000-5645fb162000 r-xp 00002000 fe:00 210247                     /usr/bin/head";
        let r = parse_region(file).unwrap();
        assert_eq!((r.start, r.len(), &r.perms), (0x5645fb15c000, 0x6000, b"r-xp"));
        assert_eq!((r.offset, r.inode, r.path), (0x2000, 210247, Some("/usr/bin/head")));

        let anon = parse_region("55d2dc98b000-55d2dc99b000 rw-p 00000000 00:00 0 ").unwrap();
        assert_eq!(anon.path, None);
        let stack = parse_region("7ffcde344000-7ffcde368000 rw-p 00000000 00:00 0      [stack]");
        assert_eq!(stack.unwrap().path, Some("[stack]"));
        let spaced = parse_region("1000-2000 r--s 00000000 00:01 7   /tmp/a b (deleted)");
        assert_eq!(spaced.unwrap().path, Some("/tmp/a b (deleted)"));
        assert_eq!(parse_region("мусор"), None);
    }

    #[test]
    fn own_mapping_is_found_by_address() {
        let m = Mapped::anonymous(1 << 20, false).unwrap();
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        let r = region_of(&maps, m.addr()).unwrap();
        // Соседнее анонимное отображение с теми же правами ядро может
        // склеить с нашим, поэтому длина — не меньше нашей.
        assert!(r.len() >= 1 << 20);
        assert_eq!(&r.perms, b"rw-p");
        let text = region_of(&maps, own_code as fn() as usize).unwrap();
        assert_eq!(&text.perms, b"r-xp");
        assert!(text.path.unwrap().contains("ch28"), "{text:?}");
    }

    fn own_code() {}

    #[test]
    fn named_mapping_if_supported() {
        let m = Mapped::anonymous(64 << 10, false).unwrap();
        if name_anonymous(&m, c"ch28-test").is_err() {
            return; // ядро без CONFIG_ANON_VMA_NAME
        }
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        let r = region_of(&maps, m.addr()).unwrap();
        assert_eq!((r.start, r.len(), r.path), (m.addr(), 64 << 10, Some("[anon:ch28-test]")));
    }

    #[test]
    fn status_and_cpu_lists() {
        let status = "Name:\tch28\nVmHWM:\t    1948 kB\nVmRSS:\t    1900 kB\nThreads:\t1\n";
        assert_eq!(status_field(status, "Name"), Some("ch28"));
        assert_eq!(status_kib(status, "VmRSS"), Some(1900));
        assert_eq!(status_kib(status, "Vm"), None);
        assert_eq!(parse_cpu_list("0-3,8,10-11\n"), Some(vec![0, 1, 2, 3, 8, 10, 11]));
        assert_eq!(parse_cpu_list("0"), Some(vec![0]));
        assert_eq!(parse_cpu_list("0-x"), None);
        let online = std::fs::read_to_string("/sys/devices/system/cpu/online").unwrap();
        assert!(!parse_cpu_list(&online).unwrap().is_empty());
    }
}
