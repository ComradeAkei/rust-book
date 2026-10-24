# 1.2. Переполнение

## Обычные операторы и профиль сборки

`+`, `-`, `*` над целыми ведут себя при переполнении так, как велит ключ
`overflow-checks` профиля (раздел [0.2](../ch00/02-cargo.md)). В `dev` он
включён, и переполнение — паника. В `release` выключен, и результат
заворачивается по модулю 2ⁿ. Неопределённым поведение не бывает ни в
одном профиле: компилятор не вправе считать, что переполнения нет, и
выкидывать проверки, которые на это опираются.

```rust
{{#include ../../code/ch01/src/overflow.rs:plain}}
```

Без проверок сложение — одна инструкция, с ними добавляется сравнение и
переход на панику:

```text
$ cargo asm -p ch01 --lib ch01_plus
ch01_plus:
	lea eax, [rdi + rsi]
	ret

$ cargo asm -p ch01 --lib --config profile.release.overflow-checks=true ch01_plus
ch01_plus:
	add edi, esi
	jb .LBB23_2
	mov eax, edi
	ret
.LBB23_2:
	push rax
	lea rdi, [rip + .Lanon.6dc6cb1b330568215e5000f29307816c.5]
	call qword ptr [rip + core::panicking::panic_const::panic_const_add_overflow@GOTPCREL]

$ cargo asm -p ch01 --lib --target thumbv7m-none-eabi \
      --config profile.release.overflow-checks=true ch01_plus
ch01_plus:
	adds r0, r0, r1
	it lo
	bxlo lr
	push {r7, lr}
	mov r7, sp
	movw r0, :lower16:.Lanon.b151a01f716457d52d813d52a532162e.5
	movt r0, :upper16:.Lanon.b151a01f716457d52d813d52a532162e.5
	bl core::panicking::panic_const::panic_const_add_overflow
```

Проверка стоит одну инструкцию условного перехода по флагу переноса: `jb`
на x86_64 (перенос есть — переход), `it lo` + `bxlo lr` на Cortex-M3
(переноса нет — возврат). Путь к панике вынесен за пределы горячего кода, и
предсказатель переходов на нём не ошибается. Поэтому проверки
переполнения иногда оставляют включёнными и в release: одна строка
`overflow-checks = true` в `[profile.release]`.

Если переполнение видно уже при компиляции, до профиля дело не доходит:

```text
error: this arithmetic operation will overflow
 --> src/main.rs:2:13
  |
2 |     let x = 255_u8 + 1;
  |             ^^^^^^^^^^ attempt to compute `u8::MAX + 1_u8`, which would overflow
  |
  = note: `#[deny(arithmetic_overflow)]` on by default
```

Линт `arithmetic_overflow` запрещён по умолчанию и видит константы,
известные компилятору до оптимизаций. Значение, пришедшее из аргумента
или через `core::hint::black_box`, он уже не видит.

## Пять семейств методов

Когда поведение при переполнении — часть логики, а не ошибка, его
выбирают явно. Метод работает одинаково в любом профиле:

```rust
{{#include ../../code/ch01/src/overflow.rs:families}}
```

| Метод | `i32::MAX + 1` | `u32::MAX + 1` | Когда нужен |
|---|---|---|---|
| `wrapping_add` | `i32::MIN` | `0` | хеши, контрольные суммы, счётчики по модулю |
| `checked_add` | `None` | `None` | данные извне, размеры буферов |
| `overflowing_add` | `(i32::MIN, true)` | `(0, true)` | длинная арифметика, нужен и результат, и признак |
| `saturating_add` | `i32::MAX` | `u32::MAX` | отсчёты АЦП, громкость, ШИМ |
| `strict_add` | паника | паника | инварианты, которые должны держаться и в release |

Такие же методы есть для `sub`, `mul`, `div`, `neg`, `pow` и `abs`, а
для сдвигов `shl` и `shr` — все, кроме насыщающих. Для частых случаев есть и типы-обёртки `Wrapping<T>` и
`Saturating<T>` из `core::num`: в них обычные операторы ведут себя как
соответствующие методы.

<div class="box ref">

**Справка: флаги переполнения и переноса.** Процессор после сложения
выставляет два разных флага. *Перенос* (CF на x86, C на ARM) — результат
не поместился в n бит как беззнаковое число. *Переполнение* (OF на x86, V
на ARM) — результат не поместился как знаковое: оба слагаемых одного
знака, а сумма другого. Это разные события:

| Сложение 32 бит | перенос | переполнение |
|---|---|---|
| `0x7FFF_FFFF + 1` | нет | да: два положительных дали отрицательное |
| `0xFFFF_FFFF + 1` | да | нет: −1 + 1 = 0 |

На x86 флаги выставляет почти любая арифметическая инструкция, кроме
`lea`. В Thumb-2 из арифметических — только те, что с суффиксом `s`:
`adds` выставляет, `add` нет (сравнения `cmp` и `tst` выставляют флаги
всегда). У вычитания ARM флаг C означает «заёма *не* было», у x86 CF —
«заём был». Код, который
переносят между архитектурами вручную, на этом ошибается; компилятор нет.

</div>

Ассемблер показывает, что семейства — не вызовы библиотеки, а
инструкции с нужным флагом:

```text
$ cargo asm -p ch01 --lib ch01_checked_add_i32
ch01_checked_add_i32:
	xor eax, eax
	add edi, esi
	setno al
	mov edx, edi
	ret

$ cargo asm -p ch01 --lib ch01_checked_add_u32
ch01_checked_add_u32:
	lea edx, [rsi + rdi]
	xor eax, eax
	cmp edx, edi
	setae al
	ret

$ cargo asm -p ch01 --lib ch01_saturating_add_u32
ch01_saturating_add_u32:
	add edi, esi
	mov eax, -1
	cmovae eax, edi
	ret
```

`Option<i32>` возвращается в двух регистрах: признак `Some` в `eax`, само
значение в `edx` (раздел [0.4](../ch00/04-abi.md)). Для знакового сложения
признак — инвертированный флаг переполнения (`setno`). Для беззнакового
LLVM выбрал другую форму: сумма меньше слагаемого тогда и только тогда,
когда был перенос, и `cmp` + `setae` проверяют именно это. Насыщение
беззнакового — `cmovae`: если переноса не было, взять сумму, иначе
`0xFFFF_FFFF`.

На Cortex-M3 те же функции используют флаги C и V через условное
выполнение:

```text
$ cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_checked_add_i32
ch01_checked_add_i32:
	push {r7, lr}
	mov r7, sp
	adds r1, r1, r0
	mov.w r0, #1
	it vc
	movvc r0, #0
	eor r0, r0, #1
	pop {r7, pc}

$ cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_saturating_add_i32
ch01_saturating_add_i32:
	push {r7, lr}
	mov r7, sp
	adds r0, r0, r1
	mov.w r1, #-2147483648
	it vs
	eorvs.w r0, r1, r0, asr #31
	pop {r7, pc}
```

Знаковое насыщение на M3 — трюк без ветвлений. При переполнении знак
суммы противоположен правильному, поэтому `r0, asr #31` даёт 0 или −1, а
`0x8000_0000 ^ 0` = `i32::MIN`, `0x8000_0000 ^ −1` = `i32::MAX`. На
Cortex-M4F для того же есть готовая инструкция из расширения DSP:

```text
$ cargo asm -p ch01 --lib --target thumbv7em-none-eabihf ch01_saturating_add_i32
ch01_saturating_add_i32:
	push {r7, lr}
	mov r7, sp
	qadd r0, r0, r1
	pop {r7, pc}
```

На Cortex-M0 (`thumbv6m`) нет ни блоков `it`, ни `qadd`, и те же функции
превращаются в условные переходы `bvc`/`blo`. Полные листинги для всех
четырёх целей снимаются командами из шапки `code/ch01/src/overflow.rs`.

<div class="box hood">

**Под капотом: две функции, один адрес.** Команда
`cargo asm -p ch01 --lib ch01_wrapping_add` ответит, что такой функции нет,
хотя в исходнике она есть и помечена `no_mangle`. Причина видна в таблице
символов объектного файла:

```console
$ readelf -sW ch01-*-cgu.0.rcgu.o | grep -E "ch01_(plus|wrapping_add)$"
    82: 0000000000000000     4 FUNC    GLOBAL DEFAULT   35 ch01_plus
    95: 0000000000000000     4 FUNC    GLOBAL DEFAULT   35 ch01_wrapping_add
```

Оба символа указывают на один и тот же адрес в одной секции. Сложение
`u32` в release и `wrapping_add` для `i32` на уровне машины — одна и та же
инструкция, и LLVM оставил одно тело с двумя именами. Знак — свойство
типа в исходнике, а не регистра.

</div>

## 128-битные целые

`u128` и `i128` — обычные типы с полным набором операций. На x86_64
значение занимает два регистра, и сложение — пара `add` и `adc` (сложение
с переносом из предыдущего разряда):

```rust
{{#include ../../code/ch01/src/wide.rs:u128}}
```

```text
$ cargo asm -p ch01 --lib ch01_add_u128
ch01_add_u128:
	mov rax, rdi
	add rax, rdx
	adc rsi, rcx
	mov rdx, rsi
	ret

$ cargo asm -p ch01 --lib ch01_sub_u128
ch01_sub_u128:
	mov rax, rdi
	sub rax, rdx
	sbb rsi, rcx
	mov rdx, rsi
	ret

$ cargo asm -p ch01 --lib --target thumbv7em-none-eabihf ch01_add_u128
ch01_add_u128:
	push {r4, r5, r6, r7, lr}
	add r7, sp, #12
	str r11, [sp, #-4]!
	add.w lr, r7, #16
	ldrd r12, r4, [r7, #8]
	ldr r5, [r7, #28]
	ldm.w lr, {r1, r6, lr}
	adds r1, r1, r2
	adcs.w r2, r6, r3
	adcs.w r3, lr, r12
	adc.w r6, r5, r4
	stm r0!, {r1, r2, r3, r6}
	ldr r11, [sp], #4
	pop {r4, r5, r6, r7, pc}
```

На Cortex-M число — четыре слова, и сложение — цепочка из четырёх
инструкций: `adds` начинает её и выставляет C, `adcs` прибавляет перенос и
передаёт его дальше, последняя `adc` перенос уже не сохраняет. Вычитание
устроено так же через `subs`/`sbcs`/`sbc`. Остальной код — доставка
аргументов: по AAPCS 16-байтный результат возвращается через память по
указателю в `r0`, первое слагаемое лежит в `r2`, `r3` и двух словах стека,
второе — целиком на стеке.

Умножение дороже. На x86_64 `ch01_mul_u128` — одна `mul` (64 × 64 → 128
бит) и две `imul` для перекрёстных произведений, восемь инструкций вместе
с `ret`. На
Cortex-M3 — 39 инструкций из `umull`, `umlal` и `mla`, а на Cortex-M0, у
которого `muls` даёт только младшие 32 бита произведения, — шесть вызовов
библиотечной `__aeabi_lmul`. Как разложить умножение на половинки вручную,
выясняет задача B1.

## Деление и сдвиги

Деление проверяется в любом профиле, независимо от `overflow-checks`.
Проверок две: деление на ноль и `i32::MIN / -1`, ведь результат 2³¹ не
помещается в `i32`:

```rust
{{#include ../../code/ch01/src/overflow.rs:div}}
```

```text
$ cargo asm -p ch01 --lib ch01_div
ch01_div:
	push rax
	test esi, esi
	je .LBB15_3
	mov eax, esi
	not eax
	lea ecx, [rdi - 2147483648]
	or ecx, eax
	je .LBB15_2
	mov eax, edi
	cdq
	idiv esi
	pop rcx
	ret
.LBB15_3:
	lea rdi, [rip + .Lanon.53a0877f8bf66fa8d8525a9866c2298c.3]
	call qword ptr [rip + core::panicking::panic_const::panic_const_div_by_zero@GOTPCREL]
.LBB15_2:
	lea rdi, [rip + .Lanon.53a0877f8bf66fa8d8525a9866c2298c.3]
	call qword ptr [rip + core::panicking::panic_const::panic_const_div_overflow@GOTPCREL]

$ cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_div
ch01_div:
	push {r7, lr}
	mov r7, sp
	cbz r1, .LBB15_3
	cmp.w r0, #-2147483648
	it eq
	addseq.w r2, r1, #1
	beq .LBB15_4
	sdiv r0, r0, r1
	pop {r7, pc}
	...
```

Вторая проверка на x86_64 собрана без ветвлений: `(a − 0x8000_0000) | !b`
равно нулю, только если a = `i32::MIN` и b = −1. Обе проверки нужны не
для красоты: на этих входах `idiv` бросает аппаратное исключение #DE, и
процесс умирает от SIGFPE. `sdiv` на Cortex-M3 и M4 не бросает ничего: деление на
ноль даёт ноль (пока сброшен бит `DIV_0_TRP` регистра CCR, а после сброса
он сброшен), а `i32::MIN / -1` даёт `i32::MIN`. У
Cortex-M0 аппаратного деления нет вовсе, и вместо `sdiv` стоит вызов
`__aeabi_idiv`. Если нужно не паниковать, есть `checked_div` (вернёт
`None`) и `wrapping_div` (вернёт `i32::MIN`).

Сдвиг на ширину типа и больше считается переполнением: в `dev` паника, в
`release` величина сдвига берётся по модулю ширины, как у `wrapping_shl`.
`1u32 << 33` в release равно 2.

```rust
{{#include ../../code/ch01/src/overflow.rs:shl}}
```

```text
$ cargo asm -p ch01 --lib ch01_shl
ch01_shl:
	mov ecx, esi
	mov eax, edi
	shl eax, cl
	ret

$ cargo asm -p ch01 --lib --target thumbv7m-none-eabi ch01_shl
ch01_shl:
	push {r7, lr}
	mov r7, sp
	and r1, r1, #31
	lsls r0, r1
	pop {r7, pc}
```

На x86_64 маскировать нечего: `shl` сам берёт из `cl` только младшие 5
бит. ARM устроен иначе: `lsls` со сдвигом из регистра берёт младший
*байт*, и сдвиг на 32–255 даёт ноль. Чтобы результат совпал с x86_64,
компилятор добавляет `and r1, r1, #31`. Это цена переносимости, и она
видна только в ассемблере.

<div class="box c">

**Как в C.** Знаковое переполнение, деление `INT_MIN / -1` и сдвиг на
ширину типа и больше — неопределённое поведение. Компилятор вправе
считать, что их не бывает: например, свернуть `x + 1 > x` в `1` для
знакового `x`. Без оптимизаций ответ определяет процессор, и он разный.
Файл `code/ch01/c/divshift.c` на x86_64 и в QEMU:

| Выражение | x86_64 | Cortex-M3 и M4F |
|---|---|---|
| `1u << 32` | 1 (`shl` маскирует сдвиг) | 0 (`lsls` берёт байт) |
| `INT_MIN / -1` | SIGFPE | `-2147483648` |
| `7 / 0` | SIGFPE | 0 |

Беззнаковое переполнение в C определено: заворачивание по модулю. Знаковое
можно сделать определённым ключом `-fwrapv`, но код, который на него
рассчитывает, ломается при сборке без ключа. Rust даёт ту же
определённость по умолчанию, а нужное поведение пишется в коде.

</div>

<div class="box trick">

**Трюк: `strict_*` как утверждение.** `strict_add` (стабилен с Rust 1.91)
паникует при переполнении в любом профиле:

```text
$ cargo asm -p ch01 --lib ch01_strict_add
ch01_strict_add:
	add edi, esi
	jb .LBB29_2
	mov eax, edi
	ret
.LBB29_2:
	push rax
	lea rdi, [rip + .Lanon.53a0877f8bf66fa8d8525a9866c2298c.5]
	call qword ptr [rip + core::num::imp::overflow_panic::add@GOTPCREL]
```

Это те же две инструкции, что и `+` с проверками, только независимо от
профиля. Им удобно отмечать места, где переполнение означает ошибку в
программе, а не в данных: смещение в буфере, номер кадра, счётчик
ссылок. Для данных извне берите `checked_*` и обрабатывайте `None`.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `checked_add` для `u32` и для `i32` смотрят на разные флаги?
2. Чему равны в release `200_u8 + 100`, `200_u8.saturating_add(100)` и
   `(-128_i8).wrapping_neg()`?
3. Зачем компилятору проверка `i32::MIN / -1`, если на Cortex-M3 `sdiv` на
   этих входах не падает?

</div>
