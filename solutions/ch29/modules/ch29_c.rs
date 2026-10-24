// SPDX-License-Identifier: GPL-2.0

//! Задача 29.C: счётчик /dev/ch29-counter, общий для всех открытий, под
//! мьютексом ядра. Поведение — как у эталона на C
//! (tests-shared/ch29/c-ref/ch29_counter_c.c).

use kernel::{
    fs::{File, Kiocb},
    iov::{IovIterDest, IovIterSource},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    new_mutex,
    prelude::*,
    sync::Mutex,
};

module! {
    type: CounterModule,
    name: "ch29_c",
    authors: ["Rust textbook"],
    description: "Task 29.C: shared counter under a kernel mutex",
    license: "GPL",
}

const MAX_WRITE: usize = 23;

// ANCHOR: global
kernel::sync::global_lock! {
    // SAFETY: инициализируется в init модуля до регистрации устройства,
    // то есть до первого open.
    unsafe(uninit) static COUNTER: Mutex<u64> = 0;
}
// ANCHOR_END: global

// ANCHOR: parse
/// Как kstrtou64(s, 10): необязательный '+', цифры, необязательный '\n'.
fn parse(s: &[u8]) -> Result<u64> {
    let s = s.strip_suffix(b"\n").unwrap_or(s);
    let s = s.strip_prefix(b"+").unwrap_or(s);
    if s.is_empty() {
        return Err(EINVAL);
    }
    let mut v: u64 = 0;
    for &c in s {
        if !c.is_ascii_digit() {
            return Err(EINVAL);
        }
        v = v.checked_mul(10).and_then(|v| v.checked_add(u64::from(c - b'0'))).ok_or(ERANGE)?;
    }
    Ok(v)
}
// ANCHOR_END: parse

#[pin_data]
struct CounterModule {
    #[pin]
    _misc: MiscDeviceRegistration<Reader>,
}

impl kernel::InPlaceModule for CounterModule {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        // SAFETY: вызывается один раз, до регистрации устройства.
        unsafe { COUNTER.init() };
        try_pin_init!(Self {
            _misc <- MiscDeviceRegistration::register(MiscDeviceOptions { name: c"ch29-counter" }),
        })
    }
}

/// Снимок значения для одного открытия: «N\n».
struct Snapshot {
    text: [u8; 24],
    len: usize,
}

#[pin_data]
struct Reader {
    #[pin]
    snap: Mutex<Snapshot>,
}

/// Пишет десятичное `v` и '\n' в начало `out`, возвращает длину.
fn format(v: u64, out: &mut [u8; 24]) -> usize {
    let mut digits = [0u8; 20];
    let (mut i, mut v) = (digits.len(), v);
    loop {
        i -= 1;
        digits[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let n = digits.len() - i;
    out[..n].copy_from_slice(&digits[i..]);
    out[n] = b'\n';
    n + 1
}

// ANCHOR: device
#[vtable]
impl MiscDevice for Reader {
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, _misc: &MiscDeviceRegistration<Self>) -> Result<Pin<KBox<Self>>> {
        KBox::try_pin_init(
            try_pin_init!(Reader { snap <- new_mutex!(Snapshot { text: [0; 24], len: 0 }) }),
            GFP_KERNEL,
        )
    }

    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        let me = kiocb.file();
        let mut snap = me.snap.lock();
        if kiocb.ki_pos() == 0 {
            let value = *COUNTER.lock();
            let mut text = [0u8; 24];
            snap.len = format(value, &mut text);
            snap.text = text;
        }
        let pos = usize::try_from(kiocb.ki_pos()).map_err(|_| EINVAL)?;
        let rest = snap.text[..snap.len].get(pos..).unwrap_or(&[]);
        if rest.is_empty() || iov.is_empty() {
            return Ok(0);
        }
        let n = iov.copy_to_iter(rest);
        if n == 0 {
            return Err(EFAULT);
        }
        *kiocb.ki_pos_mut() += n as i64;
        Ok(n)
    }

    fn write_iter(_kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterSource<'_>) -> Result<usize> {
        let len = iov.len();
        if len > MAX_WRITE {
            return Err(EINVAL);
        }
        let mut buf = [0u8; MAX_WRITE];
        if iov.copy_from_iter(&mut buf[..len]) != len {
            return Err(EFAULT);
        }
        let add = parse(&buf[..len])?;
        let mut counter = COUNTER.lock();
        *counter = counter.checked_add(add).ok_or(EOVERFLOW)?;
        Ok(len)
    }
}
// ANCHOR_END: device
