// SPDX-License-Identifier: GPL-2.0

//! Задача 29.D1: перенос samples/kobject/kobject-example.c на Rust.
//! Каталог /sys/kernel/kobject_example с файлами foo, baz и bar, как у
//! оригинала. Абстракций kobject и sysfs в крейте kernel 7.0 нет, поэтому
//! всё — через сгенерированные привязки и `unsafe`.

use core::ptr::null_mut;
use kernel::{
    bindings::{self, attribute, attribute_group, kobj_attribute, kobject},
    prelude::*,
    sync::atomic::{Atomic, Relaxed},
};

module! {
    type: KobjectExample,
    name: "ch29_d1",
    authors: ["Rust textbook"],
    description: "Task 29.D1: samples/kobject/kobject-example.c in Rust",
    license: "GPL",
}

// ANCHOR: values
// В C это три `static int` без синхронизации: show и store из разных
// процессов гоняются за ними. Здесь — атомики: гонки данных нет.
static FOO: Atomic<i32> = Atomic::new(0);
static BAZ: Atomic<i32> = Atomic::new(0);
static BAR: Atomic<i32> = Atomic::new(0);
// ANCHOR_END: values

// ANCHOR: show
/// sysfs_emit(buf, "%d\n", v). sysfs передаёт буфер в PAGE_SIZE байт.
///
/// # Safety
///
/// `buf` — буфер, который sysfs выдал для show.
unsafe fn emit(buf: *mut c_char, v: i32) -> isize {
    // SAFETY: буфер от sysfs; формат "%d\n" ждёт ровно один int — его и
    // передаём (i32 — это int на всех целях ядра).
    unsafe { bindings::sysfs_emit(buf, c"%d\n".as_ptr().cast(), v) as isize }
}

/// kstrtoint(buf, 10, &v): ошибка разбора — отрицательный код errno.
///
/// # Safety
///
/// `buf` — строка с нулём на конце, которую sysfs выдал для store.
unsafe fn parse(buf: *const c_char) -> core::result::Result<i32, isize> {
    let mut v: c_int = 0;
    // SAFETY: sysfs гарантирует, что буфер store оканчивается нулём.
    let ret = unsafe { bindings::kstrtoint(buf, 10, &mut v) };
    if ret < 0 { Err(ret as isize) } else { Ok(v) }
}

/// show для foo.
///
/// # Safety
///
/// Вызывает только sysfs: `buf` — буфер show в PAGE_SIZE байт.
unsafe extern "C" fn foo_show(_: *mut kobject, _: *mut kobj_attribute, buf: *mut c_char) -> isize {
    // SAFETY: вызывает sysfs с буфером show.
    unsafe { emit(buf, FOO.load(Relaxed)) }
}

/// store для foo.
///
/// # Safety
///
/// Вызывает только sysfs: `buf` — строка с нулём на конце.
unsafe extern "C" fn foo_store(
    _: *mut kobject,
    _: *mut kobj_attribute,
    buf: *const c_char,
    count: usize,
) -> isize {
    // SAFETY: вызывает sysfs с буфером store.
    match unsafe { parse(buf) } {
        Ok(v) => {
            FOO.store(v, Relaxed);
            count as isize
        }
        Err(e) => e,
    }
}

/// Какую переменную трогать: по имени атрибута, как b_show в C.
///
/// # Safety
///
/// `attr` — атрибут, с которым sysfs вызвал show или store.
unsafe fn which(attr: *mut kobj_attribute) -> &'static Atomic<i32> {
    // SAFETY: имя атрибута — строка с нулём на конце из статики ниже.
    let name = unsafe { CStr::from_char_ptr((*attr).attr.name) };
    if name == c"baz" { &BAZ } else { &BAR }
}

/// show для baz и bar.
///
/// # Safety
///
/// Вызывает только sysfs: `attr` — один из атрибутов модуля, `buf` — буфер
/// show.
unsafe extern "C" fn b_show(_: *mut kobject, attr: *mut kobj_attribute, buf: *mut c_char) -> isize {
    // SAFETY: вызывает sysfs со своим атрибутом и буфером show.
    unsafe { emit(buf, which(attr).load(Relaxed)) }
}

/// store для baz и bar.
///
/// # Safety
///
/// Вызывает только sysfs: `attr` — один из атрибутов модуля, `buf` — строка
/// с нулём на конце.
unsafe extern "C" fn b_store(
    _: *mut kobject,
    attr: *mut kobj_attribute,
    buf: *const c_char,
    count: usize,
) -> isize {
    // SAFETY: вызывает sysfs со своим атрибутом и буфером store.
    match unsafe { parse(buf) } {
        Ok(v) => {
            // SAFETY: как выше.
            unsafe { which(attr) }.store(v, Relaxed);
            count as isize
        }
        Err(e) => e,
    }
}
// ANCHOR_END: show

// ANCHOR: attrs
/// VERIFY_OCTAL_PERMISSIONS из include/linux/sysfs.h. Статика вычисляется
/// при компиляции, и `assert!` в const fn, вызванной из её
/// инициализатора, — ошибка компиляции, как BUILD_BUG_ON_ZERO в C.
const fn verify_octal_permissions(perms: u16) -> u16 {
    assert!(perms <= 0o777, "права больше 0777");
    assert!((perms >> 6) & 4 >= (perms >> 3) & 4, "группа читает, владелец нет");
    assert!((perms >> 3) & 4 >= perms & 4, "остальные читают, группа нет");
    assert!((perms >> 6) & 2 >= (perms >> 3) & 2, "группа пишет, владелец нет");
    assert!(perms & 2 == 0, "запись для всех в sysfs запрещена");
    perms
}

/// `__ATTR(name, mode, show, store)` — с той же проверкой прав.
const fn attr(
    name: &'static CStr,
    mode: u16,
    show: unsafe extern "C" fn(*mut kobject, *mut kobj_attribute, *mut c_char) -> isize,
    store: unsafe extern "C" fn(*mut kobject, *mut kobj_attribute, *const c_char, usize) -> isize,
) -> kobj_attribute {
    kobj_attribute {
        // Остальные поля attribute есть только в ядре с lockdep — нули.
        // В ядре без lockdep clippy сочтёт `..` лишним: полей больше нет.
        // `as_char_ptr` из CStrExt — не const fn; приведение указателя — const.
        #[allow(clippy::needless_update)]
        attr: attribute {
            name: name.as_ptr().cast(),
            mode: verify_octal_permissions(mode),
            ..pin_init::zeroed()
        },
        show: Some(show),
        store: Some(store),
    }
}

// Ядро получает на них `*mut`, и на массив атрибутов — тоже. Сами
// структуры ядро не меняет, но тип указателя требует изменяемой статики.
static mut FOO_ATTR: kobj_attribute = attr(c"foo", 0o664, foo_show, foo_store);
static mut BAZ_ATTR: kobj_attribute = attr(c"baz", 0o664, b_show, b_store);
static mut BAR_ATTR: kobj_attribute = attr(c"bar", 0o664, b_show, b_store);

static mut ATTRS: [*mut attribute; 4] = [
    // `&raw mut` на всю изменяемую статику безопасен, а на её поле — уже
    // нет (E0133): компилятор видит в `FOO_ATTR.attr` чтение статики.
    // SAFETY: берём только адрес, ссылки на статику не возникает.
    unsafe { &raw mut FOO_ATTR.attr },
    // SAFETY: как выше.
    unsafe { &raw mut BAZ_ATTR.attr },
    // SAFETY: как выше.
    unsafe { &raw mut BAR_ATTR.attr },
    null_mut(), // как в C: список кончается NULL
];

static mut GROUP: attribute_group = attribute_group {
    // В C поле attrs лежит в безымянном объединении с attrs_const; bindgen
    // называет его по порядку — второе безымянное поле структуры.
    __bindgen_anon_2: bindings::attribute_group__bindgen_ty_2 {
        // Взять адрес изменяемой статики — не unsafe: ссылки не возникает.
        attrs: (&raw mut ATTRS).cast::<*mut attribute>(),
    },
    ..pin_init::zeroed()
};
// ANCHOR_END: attrs

// ANCHOR: module
struct KobjectExample {
    kobj: *mut kobject,
}

// SAFETY: указатель на kobject живёт от init до Drop; само ядро разрешает
// kobject_put из любого потока.
unsafe impl Send for KobjectExample {}
// SAFETY: через `&self` с указателем ничего не делают.
unsafe impl Sync for KobjectExample {}

impl kernel::Module for KobjectExample {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        // SAFETY: kernel_kobj — /sys/kernel, создан при загрузке ядра.
        let parent = unsafe { bindings::kernel_kobj };
        // SAFETY: имя — строка с нулём на конце, родитель действителен.
        let kobj = unsafe { bindings::kobject_create_and_add(c"kobject_example".as_char_ptr(), parent) };
        if kobj.is_null() {
            return Err(ENOMEM);
        }
        // SAFETY: kobj создан выше; группа и атрибуты — статика модуля и
        // живут, пока модуль загружен, а Drop снимает kobject раньше.
        let ret = unsafe { bindings::sysfs_create_group(kobj, &raw const GROUP) };
        if ret != 0 {
            // SAFETY: отдаём единственную ссылку, полученную выше.
            unsafe { bindings::kobject_put(kobj) };
            return Err(Error::from_errno(ret));
        }
        Ok(KobjectExample { kobj })
    }
}

impl Drop for KobjectExample {
    fn drop(&mut self) {
        // SAFETY: ссылка из init, отдаётся один раз; файлы группы ядро
        // убирает вместе с kobject.
        unsafe { bindings::kobject_put(self.kobj) };
    }
}
// ANCHOR_END: module
