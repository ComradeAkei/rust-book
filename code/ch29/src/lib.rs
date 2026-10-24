//! Глава 29: Rust в ядре Linux.
//!
//! Модули ядра главы лежат в `modules/` и собираются Kbuild, а не cargo.
//! Этот крейт — обвязка, общая для главы, заготовок и решений задач:
//!
//! - [`host`] — на машине разработчика: собранное ядро, `make M= MO=`,
//!   initramfs через `usr/gen_init_cpio`, запуск QEMU;
//! - [`guest`] — внутри виртуальной машины: init, который монтирует
//!   /proc и /sys, грузит модули вызовом `finit_module` и читает журнал;
//! - [`kmsg`] — разбор записей `/dev/kmsg`;
//! - [`session`] — сеанс в госте с выводом, как в оболочке.
#![cfg(target_os = "linux")]

pub mod guest;
pub mod host;
pub mod kmsg;
pub mod session;
