//! Задача B: свой `Rc` со слабыми ссылками на сырых указателях.
//!
//! Блок в куче держит два счётчика и значение. Значение уничтожается,
//! когда умирает последний `MyRc`, а сам блок — когда умирает последний
//! `MyWeak`. Все сильные ссылки вместе держат одну слабую, как в `std`:
//! так блок не освободят, пока жив хоть один `MyRc`, даже если последний
//! `MyWeak` умрёт раньше. Все тесты проходят под Miri.

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

/// `MyWeak::new()` не выделяет память: у него в `ptr` висячий адрес
/// `usize::MAX`, по которому блока заведомо нет.
pub struct MyWeak<T> {
    ptr: NonNull<Inner<T>>,
}

/// Ссылки только на счётчики. Ссылка на весь `Inner` накрыла бы и
/// значение, а его в это время может уничтожать другой код: деструктор
/// значения уничтожает слабые ссылки на этот же блок (приложение E).
struct Counts<'a> {
    strong: &'a Cell<usize>,
    weak: &'a Cell<usize>,
}

/// # Safety
///
/// Блок `ptr` жив, пока живы ссылки результата.
unsafe fn counts<'a, T>(ptr: NonNull<Inner<T>>) -> Counts<'a> {
    let p = ptr.as_ptr();
    // SAFETY: блок жив по контракту; ссылки берутся только на поля
    // счётчиков, а `Cell` разрешает менять их через общую ссылку.
    unsafe { Counts { strong: &(*p).strong, weak: &(*p).weak } }
}
// ANCHOR_END: types

// ANCHOR: rc
impl<T> MyRc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(Inner {
            strong: Cell::new(1),
            weak: Cell::new(1),
            value: ManuallyDrop::new(value),
        });
        // Box::into_raw не бывает нулевым.
        Self { ptr: NonNull::from(Box::leak(inner)), _owns: PhantomData }
    }

    fn counts(&self) -> Counts<'_> {
        // SAFETY: пока жив MyRc, блок жив (strong ≥ 1 держит weak ≥ 1).
        unsafe { counts(self.ptr) }
    }

    pub fn strong_count(this: &Self) -> usize {
        this.counts().strong.get()
    }

    pub fn weak_count(this: &Self) -> usize {
        this.counts().weak.get() - 1
    }

    pub fn downgrade(this: &Self) -> MyWeak<T> {
        let weak = this.counts().weak;
        weak.set(weak.get().checked_add(1).expect("переполнение счётчика"));
        MyWeak { ptr: this.ptr }
    }

    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.ptr == b.ptr
    }

    /// `&mut T`, только если других `MyRc` и `MyWeak` нет: иначе кто-то
    /// мог бы читать значение одновременно с изменением.
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        let c = this.counts();
        if c.strong.get() == 1 && c.weak.get() == 1 {
            // SAFETY: ссылок на блок больше нет, `this` заимствован
            // изменяемо — значит, эта `&mut` единственная.
            Some(unsafe { &mut *(*this.ptr.as_ptr()).value })
        } else {
            None
        }
    }

    /// Значение, если `this` — последний `MyRc`. Слабые ссылки при этом
    /// остаются и больше не поднимутся.
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if Self::strong_count(&this) != 1 {
            return Err(this);
        }
        let this = ManuallyDrop::new(this);
        this.counts().strong.set(0);
        // SAFETY: strong = 0, и никто больше значения не увидит: upgrade
        // откажет. Значение забираем один раз.
        let value = unsafe { ManuallyDrop::take(&mut (*this.ptr.as_ptr()).value) };
        // Сильные ссылки больше не держат свою слабую.
        drop(MyWeak { ptr: this.ptr });
        Ok(value)
    }
}

impl<T> Clone for MyRc<T> {
    fn clone(&self) -> Self {
        let strong = self.counts().strong;
        strong.set(strong.get().checked_add(1).expect("переполнение счётчика"));
        Self { ptr: self.ptr, _owns: PhantomData }
    }
}

impl<T> Deref for MyRc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: блок жив; значение живо, пока жив хоть один MyRc.
        unsafe { &(*self.ptr.as_ptr()).value }
    }
}

impl<T> Drop for MyRc<T> {
    fn drop(&mut self) {
        let strong = self.counts().strong;
        strong.set(strong.get() - 1);
        if strong.get() > 0 {
            return;
        }
        // SAFETY: последний MyRc: значение больше никто не увидит.
        // Деструктор T может уничтожить последний MyWeak на этот же блок —
        // блок всё равно жив, его держит слабая ссылка сильных.
        unsafe { ManuallyDrop::drop(&mut (*self.ptr.as_ptr()).value) };
        drop(MyWeak { ptr: self.ptr });
    }
}
// ANCHOR_END: rc

// ANCHOR: weak
const DANGLING: usize = usize::MAX;

impl<T> MyWeak<T> {
    pub fn new() -> Self {
        // usize::MAX не бывает адресом блока с выравниванием больше 1, а у
        // Inner выравнивание не меньше, чем у usize.
        Self { ptr: NonNull::new(core::ptr::without_provenance_mut(DANGLING)).unwrap() }
    }

    fn counts(&self) -> Option<Counts<'_>> {
        if self.ptr.as_ptr() as usize == DANGLING {
            None
        } else {
            // SAFETY: MyWeak держит блок, пока жив.
            Some(unsafe { counts(self.ptr) })
        }
    }

    pub fn upgrade(&self) -> Option<MyRc<T>> {
        let c = self.counts()?;
        let strong = c.strong.get();
        if strong == 0 {
            return None;
        }
        c.strong.set(strong + 1);
        Some(MyRc { ptr: self.ptr, _owns: PhantomData })
    }

    pub fn strong_count(&self) -> usize {
        self.counts().map_or(0, |c| c.strong.get())
    }

    /// Как у `std`: 0, если значения уже нет.
    pub fn weak_count(&self) -> usize {
        match self.counts() {
            Some(c) if c.strong.get() > 0 => c.weak.get() - 1,
            _ => 0,
        }
    }
}

impl<T> Default for MyWeak<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for MyWeak<T> {
    fn clone(&self) -> Self {
        if let Some(c) = self.counts() {
            c.weak.set(c.weak.get().checked_add(1).expect("переполнение счётчика"));
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for MyWeak<T> {
    fn drop(&mut self) {
        let Some(c) = self.counts() else { return };
        c.weak.set(c.weak.get() - 1);
        if c.weak.get() == 0 {
            // SAFETY: последняя ссылка на блок; значение уже уничтожено
            // (strong = 0), а ManuallyDrop не уничтожит его второй раз.
            drop(unsafe { Box::from_raw(self.ptr.as_ptr()) });
        }
    }
}
// ANCHOR_END: weak
