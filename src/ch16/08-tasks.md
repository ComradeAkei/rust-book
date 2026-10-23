# 16.8. Задачи

Задачи B и C решаются кодом: заготовки — в `exercises/ch16`, тесты, общие
для заготовок и решений, — в `tests-shared/ch16.rs`. Задачи A и D
решаются не кодом, а запуском под Miri: в них неопределённое поведение
(A) или зависимость от модели (D), поэтому они живут в
`solutions/ch16/tests/ub.rs` с пометкой `#[ignore]`.

```console
$ cargo test -p ch16-exercises                 # задачи B и C
$ cargo test -p ch16-exercises b_              # одна задача
$ cargo +nightly miri test -p ch16-exercises --test ch16   # B и C под Miri
$ bash code/ch16/ub/run.sh                      # каталог A, unsound C, расхождение D
```

Пока B и C не решены, их тесты падают на `todo!()`. Разборы — в
[приложении E](../appendix-e/ch16.md), но сначала попробуйте сами.

## A. Каталог неопределённого поведения

Каталог в `solutions/ch16/tests/ub.rs` содержит фрагменты, по одному на
пункт перечня из [16.2](02-invariants.md): выход за границы,
use-after-free, недопустимое значение, нарушение алиасинга,
неинициализированная память, невыровненный доступ. Каждый — отдельный
`#[ignore]`-тест, потому что первое же UB прекращает весь процесс Miri.
Образец:

```rust
{{#include ../../solutions/ch16/tests/ub.rs:a_bounds}}
```

Запустите каждый под Miri поодиночке и убедитесь, что он сообщает об
ошибке; для каждого назовите, какой именно пункт перечня нарушен и в
какой строке Miri его ловит. Затем для двух пунктов — невыровненного
доступа и чтения за концом буфера — объясните, почему на реальном x86 они
часто «работают», а Miri их всё равно считает UB.

```console
$ cargo +nightly miri test -p ch16-solutions --test ch16-ub a_bounds -- --ignored
```

## B. split_first_mut вручную

Реализуйте `slice::split_first_mut` сами, через сырые указатели: верните
первый элемент и хвост среза как две живые непересекающиеся `&mut`.

```rust
{{#include ../../exercises/ch16/src/b_split.rs:skeleton}}
```

Весь `unsafe` обязан уместиться внутри и быть снабжён `// SAFETY`,
объясняющим, почему отрезки `[0,1)` и `[1,len)` не пересекаются. Тест
пишет через обе ссылки сразу и проверяет, что память разная; прогоните
его и под Miri в обеих моделях — сырые указатели обязаны проходить обе
([16.5](05-models.md)).

```console
$ cargo +nightly miri test -p ch16-exercises --test ch16 b_
$ MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test -p ch16-exercises --test ch16 b_
```

## C. Unsound-API и его исправление

Дан API с безопасной сигнатурой, который тем не менее *unsound*:
`'static` обещан, а строка живёт не вечно.

```rust
{{#include ../../solutions/ch16/tests/ub.rs:c_unsound}}
```

Сначала напишите программу **без единого `unsafe`**, которая через этот
API получает use-after-free (в каталоге это `c_unsound_dangles`), и
убедитесь, что Miri ловит висячую ссылку. Затем исправьте API так, чтобы
безопасный код в принципе не мог получить UB, — не починкой вызывающего,
а честной сигнатурой: время жизни результата должно быть связано с
источником. Заготовка `StrBuf` в `exercises/ch16` — место для честной
версии.

```console
$ cargo +nightly miri test -p ch16-solutions --test ch16-ub c_unsound_dangles -- --ignored
```

## D. Tree Borrows принимает, Stacked Borrows отвергает

Найдите программу, корректную под Tree Borrows и ошибочную под Stacked
Borrows, объясните разницу моделей и что она значит для автора
`unsafe`-кода. Готовый пример и его разбор — `d_tree_ok_stack_ub` в
каталоге и [приложение E](../appendix-e/ch16.md); сначала попробуйте
построить свой из правила «сырой указатель из `&mut`, затем
переодалживание той же `&mut`».

```console
$ cargo +nightly miri test -p ch16-solutions --test ch16-ub d_tree_ok_stack_ub -- --ignored
$ MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test \
      -p ch16-solutions --test ch16-ub d_tree_ok_stack_ub -- --ignored
```

<div class="box check">

**Проверь себя.**

1. В задаче A: почему невыровненное чтение `u32` на x86 обычно не падает,
   но Miri считает его UB? Что сломается на архитектуре, которая
   выравнивания требует?
2. В задаче B: почему нельзя вернуть `(&mut s[0], &mut s[1..])` без
   `unsafe` — ведь память не пересекается? Что именно не может доказать
   заёмщик?
3. В задаче C: где в unsound-варианте ошибка — в `transmute`, в сигнатуре
   или в вызывающем? Почему починка вызывающего не делает API sound?
4. В задаче D: какой retag снимает метку сырого указателя со стека в
   Stacked Borrows и почему Tree Borrows этого не делает?

</div>
