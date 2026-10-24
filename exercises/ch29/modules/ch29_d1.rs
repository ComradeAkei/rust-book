// SPDX-License-Identifier: GPL-2.0

//! Задача 29.D1: перенос samples/kobject/kobject-example.c на Rust —
//! каталог /sys/kernel/kobject_example с файлами foo, baz и bar, как у
//! оригинала. Абстракций kobject и sysfs в крейте kernel 7.0 нет:
//! понадобятся сгенерированные привязки (`kernel::bindings`) и `unsafe`
//! с комментарием-обоснованием SAFETY у каждого блока — тест собирает
//! модули с CLIPPY=1, и clippy ядра проверяет это. Заготовка грузится и
//! ничего не создаёт.

use kernel::prelude::*;

module! {
    type: KobjectExample,
    name: "ch29_d1",
    authors: ["Rust textbook"],
    description: "Task 29.D1: samples/kobject/kobject-example.c in Rust",
    license: "GPL",
}

struct KobjectExample;

impl kernel::Module for KobjectExample {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("заготовка задачи 29.D1\n");
        Ok(KobjectExample)
    }
}
