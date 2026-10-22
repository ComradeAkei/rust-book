//! Задача 3.B. Страж области видимости и транзакция над регистрами.
//!
//! Требования к `ScopeGuard`:
//! - действие выполняется ровно один раз при любом выходе из области
//!   видимости, в том числе при панике;
//! - `into_inner` возвращает значение и не выполняет действие; значение и
//!   замыкание уничтожаются ровно по разу;
//! - страж не занимает ни байта сверх значения и замыкания: никаких флагов
//!   «обезврежен» и `Option` (тест `no_extra_bytes`).
//!
//! Требования к `Transaction` — в условии задачи. Транзакцию постройте на
//! `ScopeGuard`.

use ch03::regs::{NREGS, Regs};
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

/// Значение и действие, которое выполнится над ним при выходе из области
/// видимости.
pub struct ScopeGuard<T, F: FnOnce(T)> {
    // Поля выберите сами. Это заглушка, чтобы заготовка собиралась.
    _todo: core::marker::PhantomData<(T, F)>,
}

impl<T, F: FnOnce(T)> ScopeGuard<T, F> {
    pub fn new(value: T, on_exit: F) -> Self {
        todo!()
    }

    /// Обезвредить стража: вернуть значение, не выполняя действие.
    pub fn into_inner(guard: Self) -> T {
        todo!()
    }
}

impl<T, F: FnOnce(T)> Deref for ScopeGuard<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        todo!()
    }
}

impl<T, F: FnOnce(T)> DerefMut for ScopeGuard<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        todo!()
    }
}

impl<T, F: FnOnce(T)> Drop for ScopeGuard<T, F> {
    fn drop(&mut self) {
        todo!()
    }
}

/// Транзакция над регистрами: пишет сразу, но помнит старые значения.
/// Если её не подтвердили, при уничтожении все изменения откатываются.
pub struct Transaction<'a> {
    // Поля выберите сами. Это заглушка, чтобы заготовка собиралась.
    _todo: &'a mut Regs,
}

impl<'a> Transaction<'a> {
    pub fn begin(regs: &'a mut Regs) -> Self {
        todo!()
    }

    pub fn read(&self, i: usize) -> u32 {
        todo!()
    }

    pub fn write(&mut self, i: usize, value: u32) {
        todo!()
    }

    /// Подтвердить: изменения остаются, откат не выполняется.
    pub fn commit(self) {
        todo!()
    }
}
