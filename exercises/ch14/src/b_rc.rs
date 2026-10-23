//! Задача B: свой `Rc` со слабыми ссылками на сырых указателях.
//!
//! Типы даны, тела — `todo!()`. Блок в куче держит два счётчика и
//! значение. Значение уничтожается, когда умирает последний `MyRc`, а сам
//! блок — когда умирает последний `MyWeak`. Подумайте, что будет, если
//! деструктор значения уничтожит последний `MyWeak` на этот же блок.
//!
//! Нативные тесты ловят не всё: прогоните их под Miri. Если Miri
//! жалуется на ссылку, которой «не дают доступа», посмотрите, на какие
//! байты она указывает и кто в это время держит `&mut` на те же байты.

use alloc::boxed::Box;
use core::cell::Cell;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use core::ptr::NonNull;

// ANCHOR: types
struct Inner<T> {
    strong: Cell<usize>,
    /// Число `MyWeak` плюс одна на все `MyRc` вместе.
    weak: Cell<usize>,
    /// Уничтожается вручную, когда `strong` дойдёт до нуля.
    value: ManuallyDrop<T>,
}

pub struct MyRc<T> {
    ptr: NonNull<Inner<T>>,
    /// `MyRc<T>` владеет `T`. Пока у `Drop` нет `#[may_dangle]`, проверку
    /// уничтожения обеспечивает и сам `Drop`; `Rc` в `std` с `may_dangle`
    /// без такого поля пропустил бы висячие ссылки внутри `T`.
    _owns: PhantomData<T>,
}

/// `MyWeak::new()` не выделяет память: блока у него нет. Как отличить
/// такой `MyWeak` от настоящего, не тратя лишнего слова, решите сами.
pub struct MyWeak<T> {
    ptr: NonNull<Inner<T>>,
}
// ANCHOR_END: types

impl<T> MyRc<T> {
    pub fn new(value: T) -> Self {
        todo!()
    }

    pub fn strong_count(this: &Self) -> usize {
        todo!()
    }

    pub fn weak_count(this: &Self) -> usize {
        todo!()
    }

    pub fn downgrade(this: &Self) -> MyWeak<T> {
        todo!()
    }

    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        todo!()
    }

    /// `&mut T`, только если других `MyRc` и `MyWeak` нет.
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        todo!()
    }

    /// Значение, если `this` — последний `MyRc`. Слабые ссылки при этом
    /// остаются и больше не поднимутся.
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        todo!()
    }
}

impl<T> Clone for MyRc<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<T> Deref for MyRc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        todo!()
    }
}

impl<T> Drop for MyRc<T> {
    fn drop(&mut self) {
        todo!()
    }
}

impl<T> MyWeak<T> {
    pub fn new() -> Self {
        todo!()
    }

    pub fn upgrade(&self) -> Option<MyRc<T>> {
        todo!()
    }

    pub fn strong_count(&self) -> usize {
        todo!()
    }

    /// Как у `std`: 0, если значения уже нет.
    pub fn weak_count(&self) -> usize {
        todo!()
    }
}

impl<T> Default for MyWeak<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for MyWeak<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl<T> Drop for MyWeak<T> {
    fn drop(&mut self) {
        todo!()
    }
}
