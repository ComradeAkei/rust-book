//! Раздел A.4: управление потоком.

use std::cell::RefCell;

/// Журнал событий, по которому тесты проверяют порядок захвата и
/// освобождения ресурсов.
pub type Log = RefCell<Vec<String>>;

// ANCHOR: cleanup
/// Ресурс драйвера: тактирование, канал DMA, линия прерывания.
pub struct Resource<'a> {
    name: &'static str,
    log: &'a Log,
}

impl Drop for Resource<'_> {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("free {}", self.name));
    }
}

fn acquire<'a>(
    name: &'static str,
    log: &'a Log,
    fail: Option<&str>,
) -> Result<Resource<'a>, String> {
    if fail == Some(name) {
        return Err(format!("{name}: занято"));
    }
    log.borrow_mut().push(format!("get {name}"));
    Ok(Resource { name, log })
}

/// Устройство владеет всеми тремя ресурсами.
pub struct Device<'a> {
    _clk: Resource<'a>,
    _dma: Resource<'a>,
    _irq: Resource<'a>,
}

/// Вместо `goto err_dma; ... err_dma: free_clk();` из C: при ошибке `?`
/// возвращает управление, и уже захваченные ресурсы освобождаются в
/// обратном порядке их создания.
pub fn init<'a>(log: &'a Log, fail: Option<&str>) -> Result<Device<'a>, String> {
    let clk = acquire("clk", log, fail)?;
    let dma = acquire("dma", log, fail)?;
    let irq = acquire("irq", log, fail)?;
    Ok(Device { _clk: clk, _dma: dma, _irq: irq })
}
// ANCHOR_END: cleanup

// ANCHOR: labeled
/// Поиск в матрице с выходом из обоих циклов сразу. В C для этого нужен
/// `goto` или флаг, здесь блок с меткой возвращает значение.
pub fn find(m: &[[i32; 4]], target: i32) -> Option<(usize, usize)> {
    'search: {
        for (r, row) in m.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                if v == target {
                    break 'search Some((r, c));
                }
            }
        }
        None
    }
}
// ANCHOR_END: labeled

// ANCHOR: do_while
/// Бит готовности в регистре состояния.
pub const READY: u32 = 1 << 0;

/// `do { s = read(); } while (!(s & READY)); return s;` — регистр читается
/// хотя бы один раз, `loop` с `break значение` возвращает последнее чтение.
pub fn wait_ready(mut read: impl FnMut() -> u32) -> u32 {
    loop {
        let s = read();
        if s & READY != 0 {
            break s;
        }
    }
}
// ANCHOR_END: do_while

#[cfg(test)]
mod tests {
    use super::*;

    fn events(log: &Log) -> Vec<String> {
        log.borrow().clone()
    }

    #[test]
    fn cleanup_in_reverse_order() {
        let log = Log::default();
        assert!(init(&log, Some("irq")).is_err());
        assert_eq!(events(&log), ["get clk", "get dma", "free dma", "free clk"]);
    }

    #[test]
    fn device_owns_resources() {
        let log = Log::default();
        let dev = init(&log, None).unwrap();
        assert_eq!(events(&log).len(), 3);
        drop(dev);
        // Поля структуры освобождаются в порядке объявления.
        assert_eq!(events(&log)[3..], ["free clk", "free dma", "free irq"]);
    }

    #[test]
    fn labeled_block() {
        let m = [[1, 2, 3, 4], [5, 6, 7, 8]];
        assert_eq!(find(&m, 7), Some((1, 2)));
        assert_eq!(find(&m, 9), None);
    }

    #[test]
    fn do_while_reads_at_least_once() {
        let mut reads = [0, 0, 3].into_iter();
        let mut count = 0;
        let s = wait_ready(|| {
            count += 1;
            reads.next().unwrap()
        });
        assert_eq!((s, count), (3, 3));
    }
}
