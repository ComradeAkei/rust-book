//! Задача B: таблица методов вручную, как `struct file_operations`.
//!
//! Эталонное решение. Таблица строится для каждого типа при компиляции
//! блоком `const { … }`, функции таблицы — копии обобщённых переходников.

use alloc::alloc::{Layout, dealloc};
use alloc::boxed::Box;
use core::marker::PhantomData;

// ANCHOR: sensor
pub trait Sensor {
    fn read(&self) -> i32;
    fn name(&self) -> &'static str {
        "датчик"
    }
}

pub struct Thermo {
    pub raw: i16,
}

impl Sensor for Thermo {
    fn read(&self) -> i32 {
        i32::from(self.raw) * 100 / 16
    }

    fn name(&self) -> &'static str {
        "термодатчик"
    }
}

pub struct Counter {
    pub pulses: u32,
}

impl Sensor for Counter {
    fn read(&self) -> i32 {
        self.pulses as i32
    }
}
// ANCHOR_END: sensor

// ANCHOR: vtable
/// Таблица в духе `file_operations`: те же слова в том же порядке, что
/// rustc 1.97 кладёт в таблицу `dyn Sensor`.
#[repr(C)]
pub struct SensorVTable {
    /// `drop_in_place` для типа; `None`, если деструктора нет.
    pub drop: Option<unsafe fn(*mut ())>,
    pub size: usize,
    pub align: usize,
    pub read: unsafe fn(*const ()) -> i32,
    pub name: unsafe fn(*const ()) -> &'static str,
}

/// Толстый указатель вручную: адрес данных и таблица. `repr(C)` нужен
/// для того же кода, что у `&dyn`: в `repr(Rust)` rustc ставит первой
/// ссылку на таблицу (у неё есть ниша — запрещённый ноль), и слова
/// приходят в регистрах в обратном порядке.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SensorRef<'a> {
    data: *const (),
    vtable: &'static SensorVTable,
    _borrow: PhantomData<&'a ()>,
}

/// Владеющий указатель, как `Box<dyn Sensor>`.
pub struct SensorBox {
    data: *mut (),
    vtable: &'static SensorVTable,
}
// ANCHOR_END: vtable

// ANCHOR: build
unsafe fn read_shim<T: Sensor>(p: *const ()) -> i32 {
    // SAFETY: таблицу для T получают только указатели на T.
    unsafe { (*p.cast::<T>()).read() }
}

unsafe fn name_shim<T: Sensor>(p: *const ()) -> &'static str {
    // SAFETY: как в read_shim.
    unsafe { (*p.cast::<T>()).name() }
}

unsafe fn drop_shim<T>(p: *mut ()) {
    // SAFETY: вызывающий отдаёт живой объект T, которым больше не пользуется.
    unsafe { p.cast::<T>().drop_in_place() }
}

/// Таблица для `T`: константа, одна на тип.
fn vtable_for<T: Sensor>() -> &'static SensorVTable {
    const {
        &SensorVTable {
            drop: if core::mem::needs_drop::<T>() { Some(drop_shim::<T>) } else { None },
            size: size_of::<T>(),
            align: align_of::<T>(),
            read: read_shim::<T>,
            name: name_shim::<T>,
        }
    }
}
// ANCHOR_END: build

// ANCHOR: api
impl<'a> SensorRef<'a> {
    pub fn new<T: Sensor>(x: &'a T) -> Self {
        SensorRef { data: (x as *const T).cast(), vtable: vtable_for::<T>(), _borrow: PhantomData }
    }

    pub fn read(self) -> i32 {
        // SAFETY: data и vtable получены вместе в new или as_ref, заём жив.
        unsafe { (self.vtable.read)(self.data) }
    }

    pub fn name(self) -> &'static str {
        // SAFETY: как в read.
        unsafe { (self.vtable.name)(self.data) }
    }

    pub fn size(self) -> usize {
        self.vtable.size
    }

    pub fn align(self) -> usize {
        self.vtable.align
    }

    pub fn vtable(self) -> &'static SensorVTable {
        self.vtable
    }
}

impl SensorBox {
    pub fn new<T: Sensor + 'static>(x: T) -> Self {
        SensorBox { data: Box::into_raw(Box::new(x)).cast(), vtable: vtable_for::<T>() }
    }

    pub fn as_ref(&self) -> SensorRef<'_> {
        SensorRef { data: self.data, vtable: self.vtable, _borrow: PhantomData }
    }
}

impl Drop for SensorBox {
    fn drop(&mut self) {
        let vt = self.vtable;
        // SAFETY: объект создан Box::new в SensorBox::new с размером и
        // выравниванием из той же таблицы; после drop к нему не обращаются.
        unsafe {
            if let Some(drop) = vt.drop {
                drop(self.data);
            }
            if vt.size != 0 {
                dealloc(self.data.cast(), Layout::from_size_align_unchecked(vt.size, vt.align));
            }
        }
    }
}
// ANCHOR_END: api

// ANCHOR: compare
/// Вызов через таблицу, построенную вручную.
#[unsafe(no_mangle)]
pub fn ch11_b_read_raw(s: SensorRef<'_>) -> i32 {
    s.read()
}

/// Тот же вызов через `dyn`.
#[unsafe(no_mangle)]
pub fn ch11_b_read_dyn(s: &dyn Sensor) -> i32 {
    s.read()
}
// ANCHOR_END: compare
