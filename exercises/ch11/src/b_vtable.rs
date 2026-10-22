//! Задача B: таблица методов вручную, как `struct file_operations`.
//!
//! `SensorRef` — толстый указатель, собранный руками: адрес данных и
//! ссылка на таблицу `SensorVTable`. `SensorBox` владеет объектом в куче и
//! освобождает его через таблицу, как `Box<dyn Sensor>`. Таблица для
//! каждого типа должна строиться при компиляции, одна на тип.

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

/// Толстый указатель вручную: адрес данных и таблица.
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

// ANCHOR: api
impl<'a> SensorRef<'a> {
    pub fn new<T: Sensor>(x: &'a T) -> Self {
        todo!("адрес x и таблица для T, построенная при компиляции")
    }

    pub fn read(self) -> i32 {
        todo!("вызов через таблицу")
    }

    pub fn name(self) -> &'static str {
        todo!()
    }

    pub fn size(self) -> usize {
        todo!()
    }

    pub fn align(self) -> usize {
        todo!()
    }

    pub fn vtable(self) -> &'static SensorVTable {
        todo!()
    }
}

impl SensorBox {
    pub fn new<T: Sensor + 'static>(x: T) -> Self {
        todo!("объект в куче, таблица для T")
    }

    pub fn as_ref(&self) -> SensorRef<'_> {
        todo!()
    }
}

impl Drop for SensorBox {
    fn drop(&mut self) {
        todo!("деструктор из таблицы, затем dealloc с размером и выравниванием из неё")
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
