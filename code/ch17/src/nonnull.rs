//! Раздел 17.3: `NonNull<T>` и `PhantomData`. `NonNull<T>` — это `*mut T`,
//! про который тип знает две вещи: он не `null` и ковариантен по `T`. На
//! этом строят умные указатели. `PhantomData<T>` добавляет к указателю
//! сведения, которых в нём физически нет: что тип владеет `T`, с каким
//! временем жизни связан, какова его вариантность.

use core::marker::PhantomData;
use core::ptr::NonNull;

// ANCHOR: niche
/// Ниша «не null» позволяет `Option<NonNull<T>>` занимать столько же
/// места, сколько сам указатель: `None` — это нулевой битовый образец,
/// который `NonNull` иметь не может.
pub fn niche_same_size() -> bool {
    size_of::<Option<NonNull<u8>>>() == size_of::<NonNull<u8>>()
}
// ANCHOR_END: niche

// ANCHOR: phantom
/// Разделяемый указатель-скелет. Поле — только `NonNull<T>`, но
/// `PhantomData<T>` говорит компилятору: тип владеет `T` (ковариантен по
/// нему и роняет его при сбросе, как `Box`), хотя значения `T` в структуре
/// нет. Без `PhantomData` автотрейты и проверка времён жизни рассуждали бы
/// так, будто `T` тут ни при чём.
pub struct Shared<T> {
    ptr: NonNull<T>,
    _own: PhantomData<T>,
}

impl<T> Shared<T> {
    /// Обернуть изменяемую ссылку: `NonNull::from` никогда не `null`.
    pub fn new(r: &mut T) -> Self {
        Shared { ptr: NonNull::from(r), _own: PhantomData }
    }

    /// Прочитать через указатель.
    ///
    /// # Safety
    ///
    /// Исходный `T` жив, и на время заёма к нему не обращаются по другой
    /// изменяемой ссылке.
    pub unsafe fn get(&self) -> &T {
        // SAFETY: ptr получен из живой &mut T; по предусловию он ещё жив и
        // к нему нет конфликтующего доступа.
        unsafe { self.ptr.as_ref() }
    }
}
// ANCHOR_END: phantom

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_nonnull_has_niche() {
        assert!(niche_same_size());
    }

    #[test]
    fn shared_reads_through_pointer() {
        let mut v = 42u32;
        let s = Shared::new(&mut v);
        // SAFETY: v жив всю область видимости, другого доступа нет.
        assert_eq!(unsafe { *s.get() }, 42);
    }
}
