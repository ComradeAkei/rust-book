# 26.6. Тесты no_std-кода на хосте

## Тест собирается со std

Библиотека `no_std` тестируется обычным `cargo test` на хосте — и это
главная причина держать логику прошивки в библиотеке, а не в `main`.
Тестовая программа — не `no_std`: обвязка `test`, которая запускает
тесты, печатает отчёт и ловит панику, сама зависит от `std`. Но в
область видимости крейта `std` от этого не попадает:

```rust,ignore
#![no_std]

pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn adds() {
        let v = vec![super::add(2, 2)];
        assert_eq!(v[0], 4);
    }
}
```

```text
error: cannot find macro `vec` in this scope
  --> fragments/vec_in_test.rs:13:17
   |
13 |         let v = vec![super::add(2, 2)];
   |                 ^^^
```

`assert_eq!` нашёлся — он в `core`, а `vec!` — нет. Модуль тестов
подключает `std` сам, двумя строками:

```rust,ignore
{{#include ../../code/ch26/src/layers.rs:tests}}
```

`extern crate std` внутри `mod tests` — только для тестов: в сборке без
`cfg(test)` модуля нет, и библиотека остаётся `no_std`. Так устроены
тесты всех `no_std`-крейтов этой книги.

## Обработчик паники — в программе

Обработчик паники в библиотеке ломает её тесты:

```rust,ignore
{{#include ../../code/ch26/fragments/panic_in_lib.rs:panic_in_lib}}
```

```text
error[E0152]: found duplicate lang item `panic_impl`
 --> fragments/panic_in_lib.rs:6:1
  |
6 | / fn panic(_: &core::panic::PanicInfo) -> ! {
7 | |     loop {}
8 | | }
  | |_^
  |
  = note: the lang item is first defined in crate `std` (which `test` depends on)
```

Обработчик паники в программе один, а `std` уже принесла свой.
`#[cfg(not(test))]` на обработчике спасёт [модульные тесты][g-unit-test] самой
библиотеки, но не [интеграционные тесты][g-integration-test] в `tests/` и не другой крейт,
который захочет её тестировать: им библиотека достаётся собранной без
`cfg(test)`, вместе с обработчиком. Обработчику место в крейте
программы — `firmware/…/main.rs`, как во всех прошивках книги.

Исключение — `ch26-bare`: её библиотека и есть среда выполнения, с
таблицей векторов и `Reset`, и на хосте её не собирают вовсе. Код,
который стоит тестировать на хосте, живёт отдельно — в `code/ch26`,
`telemetry-frame` и решениях задач.

## Возможности вместо cfg цели

Что нужно хосту, а не прошивке, подключают возможности крейта
(раздел [26.1](01-layers.md)): `std` по умолчанию для тестов и
примеров, `default-features = false` в прошивке. Проверять цель —
`#[cfg(target_os = "none")]` — стоит только там, где различие
действительно в среде, а не в наборе библиотек: так решение задачи D
экспортирует свои `memcpy` под именем C только на голом железе
(раздел [26.5](05-memcpy.md)).

CI проверяет обе стороны: `cargo test` на хосте со всеми возможностями
и `cargo build --no-default-features` под три цели Cortex-M — тот же
код без `std`. А системные вызовы из `sys` собираются только для
`target_os = "linux"` и `target_arch = "x86_64"`, и на Cortex-M модуля
нет вовсе.

## Miri

Miri (раздел [16.5](../ch16/05-models.md)) выполняет `no_std`-код так
же, как любой другой: `Cursor` поверх среза, функции раздела 26.5 и
свои `memcpy`, `memmove` и `memset` из задачи D — на всех выравниваниях
и перекрытиях, в обеих моделях [алиасинга][g-aliasing]. Не выполняет он только
ассемблер: тесты `sys` помечены `#[cfg_attr(miri, ignore)]`, и проверяют
их обычные прогоны.

```console
$ cargo +nightly miri test -p ch26 -p ch26-solutions
$ MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test -p ch26 -p ch26-solutions
```

То, что зависит от железа, — таблица векторов, код запуска, запись во
flash — проверяет QEMU: `firmware/run-demos.sh` запускает прошивки
главы на трёх ядрах и сверяет вывод, а сборка, которая должна упасть,
проверяется по тексту ошибки.

<div class="box check">

**Проверь себя.** Библиотека `no_std` объявляет `#[global_allocator]`
для прошивки. Что случится с её тестами на хосте? Где этот атрибут
должен стоять?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-aliasing]: ../appendix-f.md#aliasing "Алиасинг (aliasing): Два пути к одной памяти: два указателя, ссылка и указатель."
[g-integration-test]: ../appendix-f.md#integration-test "Интеграционный тест (integration test): Файл прямо в каталоге tests/ пакета: отдельный крейт, который зависит от библиотеки как обычный пользователь и видит только её публичный API."
[g-unit-test]: ../appendix-f.md#unit-test "Модульный тест (unit test): Тест в том же файле, что и код: функция #[test] в модуле под #[cfg(test)]."
