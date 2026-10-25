# 12.5. Замыкание в машинном коде

## Окружение — первый аргумент

Обобщённая функция с параметром `F: Fn(u32) -> u32` получает по копии
на каждое замыкание (мономорфизация, глава 10). Структура замыкания
приходит в копию как обычный аргумент, и то, что в ней лежит, видно по
регистрам:

```rust
{{#include ../../code/ch12/src/codegen.rs:env}}
```

```console
$ cargo asm -p ch12 --lib --simplify "ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_zst::{closure#0}>"
ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_zst::{closure#0}>:
	lea eax, [rdi + 8*rdi]
	ret
```

```console
$ cargo asm -p ch12 --lib --simplify "ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_ref::{closure#0}>"
ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_ref::{closure#0}>:
	mov eax, dword ptr [rdi]
	imul esi, eax
	imul eax, esi
	ret
```

```console
$ cargo asm -p ch12 --lib --simplify "ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_val::{closure#0}>"
ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_val::{closure#0}>:
	mov eax, edi
	imul eax, edi
	imul eax, esi
	ret
```

Три копии одной функции, и в каждой своё окружение:

- Без захвата структура пуста, и `x` приходит первым аргументом в
  `edi`. Два умножения на три LLVM свернул в одно на девять: `lea` с
  масштабом 8 считает `x + 8x`.
- С захватом по ссылке первым аргументом идёт указатель на `k` (`rdi`),
  а `x` сдвигается во второй (`esi`). `k` читается из памяти один раз на
  два вызова: ссылка `&u32` неизменяема, пока жив заём, и LLVM об этом
  знает.
- С захватом по значению первым аргументом идёт само `k` (`edi`): u32
  в регистре, без обращения к памяти.

То же на Cortex-M3: окружение в `r0`, аргумент в `r1`.

```console
$ cargo asm -p ch12 --lib --simplify --no-default-features --target thumbv7m-none-eabi "ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_ref::{closure#0}>"
ch12::codegen::apply_twice::<ch12::codegen::ch12_twice_ref::{closure#0}>:
	push {r7, lr}
	mov r7, sp
	ldr r0, [r0]
	muls r1, r0, r1
	muls r0, r1, r0
	pop {r7, pc}
```

`#[inline(never)]` здесь только для того, чтобы копии остались видны.
Без него каждая встроилась бы в вызывающего, и `ch12_twice_ref` стала
бы тремя инструкциями.

## Встраивание: замыкание исчезает

Калибровка из введения на Rust — обобщённая функция и замыкание с
захватом `&Affine`:

```rust
{{#include ../../code/ch12/src/codegen.rs:map}}
```

```console
$ cargo asm -p ch12 --lib --simplify ch12_scale_generic
ch12_scale_generic:
	test rsi, rsi
	je .LBB41_7
	lea r8, [rsi + rsi]
	movzx eax, word ptr [rdx]
	movzx ecx, word ptr [rdx + 2]
…
.LBB41_9:
	movdqu xmm2, xmmword ptr [rdi + 2*r9]
	movdqu xmm3, xmmword ptr [rdi + 2*r9 + 16]
	pmullw xmm2, xmm0
	pmullw xmm3, xmm0
	paddw xmm2, xmm1
	paddw xmm3, xmm1
	movdqu xmmword ptr [rdi + 2*r9], xmm2
	movdqu xmmword ptr [rdi + 2*r9 + 16], xmm3
	add r9, 16
	cmp r8, r9
	jne .LBB41_9
…
```

От замыкания не осталось ничего: ни структуры, ни вызова. `k` и `b`
читаются из `[rdx]` и `[rdx + 2]` один раз до цикла, и основной цикл
обрабатывает шестнадцать отсчётов за проход двумя парами `pmullw` и
`paddw`; хвосты по четыре и по одному отсчёту идут дальше.

В C ту же функцию пришлось написать отдельно, а `k` и `b` вынести в
локальные переменные руками: `int16_t *a` и `const struct affine *p`
могут указывать на одну память, и компилятор обязан перечитывать
`p->k` после каждой записи в `a[i]`. В Rust `&mut [i16]` и `&Affine` не
пересекаются по правилам заимствования (глава 4), rustc передаёт это
LLVM как `noalias`, и вынос из цикла получается сам.

На Cortex-M3 векторных инструкций нет, и цикл развёрнут вчетверо, по
одному `mla` (умножение со сложением) на отсчёт:

```console
$ cargo asm -p ch12 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch12_scale_generic
ch12_scale_generic:
	cmp r1, #0
	it eq
	bxeq lr
	push {r4, r6, r7, lr}
	add r7, sp, #8
	lsl.w r12, r1, #1
	ldrh r4, [r2, #2]
	ldrh r2, [r2]
…
.LBB23_7:
	ldrh r3, [r1, #8]!
	mla r3, r3, r2, r4
	strh r3, [r1]
	ldrh r3, [r1, #2]
	mla r3, r3, r2, r4
	strh r3, [r1, #2]
	ldrh r3, [r1, #4]
	mla r3, r3, r2, r4
	strh r3, [r1, #4]
	ldrh r3, [r1, #6]
	mla r3, r3, r2, r4
	strh r3, [r1, #6]
	add.w r3, r1, #8
	cmp r3, r0
	bne .LBB23_7
…
```

## `&mut dyn FnMut`: одна копия и косвенный вызов

Если копию на каждое замыкание иметь не хочется — функция большая,
замыканий много или функция должна жить в другом крейте, — тип
стирают:

```rust
{{#include ../../code/ch12/src/codegen.rs:dyn}}
```

Вызывающий строит структуру замыкания у себя на стеке и передаёт
толстый указатель — адрес структуры и адрес таблицы:

```console
$ cargo asm -p ch12 --lib --simplify ch12_scale_dyn 2
ch12_scale_dyn:
	push rax
	mov qword ptr [rsp], rdx
	lea rcx, [rip + .Lanon.51476fdeced9374820a5d9675d9fc4c4.6]
	mov rdx, rsp
	call qword ptr [rip + ch12_map_dyn@GOTPCREL]
	pop rax
	ret
```

Структура — восемь байт на стеке с единственным полем, `p`. Тело
замыкания стало отдельной функцией, которая получает адрес структуры
первым аргументом и достаёт `p` из неё:

```console
$ cargo asm -p ch12 --lib --simplify "ch12::codegen::ch12_scale_dyn::{closure#0}" 1
ch12::codegen::ch12_scale_dyn::{closure#0}:
	mov eax, esi
	mov rcx, qword ptr [rdi]
	imul ax, word ptr [rcx]
	add ax, word ptr [rcx + 2]
	ret
```

А цикл зовёт её на каждый отсчёт:

```console
$ cargo asm -p ch12 --lib --simplify ch12_map_dyn
ch12_map_dyn:
	test rsi, rsi
	je .LBB38_4
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov rbx, rdx
	mov r14, rsi
	mov r15, rdi
	add r14, rsi
	mov r12, qword ptr [rcx + 32]
	xor r13d, r13d
.LBB38_2:
	movzx esi, word ptr [r15 + r13]
	mov rdi, rbx
	call r12
	mov word ptr [r15 + r13], ax
	add r13, 2
	cmp r14, r13
	jne .LBB38_2
…
```

Таблица `dyn FnMut(i16) -> i16` устроена как таблица любого `dyn` из
главы 11: деструктор, размер, выравнивание, затем методы трейта и его
супертрейтов. Слот 3 (смещение 24) — `call_once` из `FnOnce`, слот 4
(смещение 32) — `call_mut`. Адрес `call_mut` загружается один раз до
цикла, потому что таблица неизменна, а сам вызов остаётся в цикле: это
`call r12`, тот же `call r13`, что GCC оставил в `apply`. На Cortex-M3
слоты по четыре байта, и `call_mut` лежит по смещению 16:

```console
$ cargo asm -p ch12 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch12_map_dyn
ch12_map_dyn:
	cmp r1, #0
	it eq
	bxeq lr
	push {r4, r5, r6, r7, lr}
	add r7, sp, #12
	str r8, [sp, #-4]!
	ldr r4, [r3, #16]
	subs r6, r0, #2
	mov r8, r2
	lsls r5, r1, #1
.LBB20_2:
	ldrh r1, [r6, #2]!
	mov r0, r8
	blx r4
	subs r5, #2
	strh r0, [r6]
	bne .LBB20_2
…
```

Указатель на функцию обходится без окружения и без таблицы, но вызов
тот же:

```rust
{{#include ../../code/ch12/src/codegen.rs:fnptr}}
```

```console
$ cargo asm -p ch12 --lib --simplify ch12_map_fn
…
.LBB39_2:
	movzx edi, word ptr [r15 + r12]
	call rbx
	mov word ptr [r15 + r12], ax
	add r12, 2
	cmp r14, r12
	jne .LBB39_2
…
```

## Цена косвенного вызова

Пример `code/ch12/examples/bench.rs` калибрует 4096 отсчётов тремя
способами и сверяет, что результаты совпали:

```console
$ cargo run --release -p ch12 --example bench
обобщённая:     0.05 нс на отсчёт
&mut dyn FnMut: 1.25 нс на отсчёт
fn-указатель:   1.25 нс на отсчёт
```

| Способ | Копий функции | Вызов на отсчёт | Время на отсчёт |
|---|---|---|---|
| `F: FnMut` | одна на замыкание | нет, тело встроено | 0.05–0.10 нс |
| `&mut dyn FnMut` | одна на всех | `call` через регистр | 1.2–1.7 нс |
| `fn(i16) -> i16` | одна на всех | `call` через регистр | 1.2–1.3 нс |
| C, `apply` с `noipa` | одна на всех | `call` через регистр | 1.3–1.4 нс |

Разница в двадцать раз — не цена косвенного вызова как такового: сам
`call` по предсказанному адресу стоит несколько тактов. Это цена
упущенной векторизации. Встроенное тело позволяет обработать
шестнадцать отсчётов за проход, а через вызов — по одному. Если тело
замыкания большое, а вызовов немного, `dyn` обходится почти бесплатно и
экономит flash; если тело — пара инструкций в горячем цикле, как здесь,
нужна обобщённая функция.

<div class="box trick">

**Трюк.** Выбирать между двумя видами не обязательно раз и навсегда.
Обобщённая функция-обёртка, которая сразу передаёт `&mut f` во
внутреннюю функцию с `&mut dyn FnMut`, даёт одну копию большого тела и
дешёвый интерфейс с `impl FnMut` для вызывающих. Тот же приём
стандартная библиотека применяет к другим обобщениям: `std::fs::read`
принимает `impl AsRef<Path>`, а всё остальное делает внутренняя функция
от `&Path` в одной копии. Обратный ход тоже есть: если горячий цикл зовёт `dyn`, а
типов на самом деле два-три, `match` по перечислению с вызовом
конкретных функций вернёт встраивание (глава 11, раздел 11.7).

</div>

## `Box<dyn FnOnce>`: вызов и освобождение

```rust
{{#include ../../code/ch12/src/codegen.rs:once}}
```

```console
$ cargo asm -p ch12 --lib --simplify ch12_call_boxed
ch12_call_boxed:
	push r15
	push r14
	push rbx
	mov r14, rsi
	mov rbx, rdi
	call qword ptr [rsi + 24]
	mov rsi, qword ptr [r14 + 8]
	test rsi, rsi
	je .LBB36_3
	mov rdx, qword ptr [r14 + 16]
	mov rdi, rbx
	mov ebx, eax
	call qword ptr [rip + __rustc::__rust_dealloc@GOTPCREL]
	mov eax, ebx
.LBB36_3:
	pop rbx
	pop r14
	pop r15
	ret
	mov r15, rax
…
	call _Unwind_Resume@PLT
```

Вызов идёт через слот 3, `call_once`: переходник получает адрес
структуры в куче и забирает её содержимое себе — по значению, как
требует `FnOnce`. Деструктор из слота 0 не вызывается: значение уже
перемещено, и уничтожает его тело замыкания. Остаётся память, и её
освобождает вызывающий: размер из слота 1 (`[r14 + 8]`) и выравнивание
из слота 2 (`[r14 + 16]`) — ровно те аргументы, которых ждёт
`__rust_dealloc`. Если размер нулевой, памяти не выделяли, и
`test rsi, rsi` обходит освобождение: `Box::new(|| 7)` кучу не трогает.
Последние строки — путь раскрутки: если замыкание запаникует, память
всё равно освободится, и паника пойдёт дальше через `_Unwind_Resume`.

Тесты `rustc_*` в `code/ch12/tests/asm.rs` проверяют ключевые
инструкции из этого раздела при каждой сборке: смещения слотов,
отсутствие вызовов в обобщённой версии, `__rust_dealloc` после
`call_once`, `mla` и `blx` на Cortex-M3.

<div class="box check">

**Проверь себя.** Почему в копии для `ch12_twice_val` нет обращений к
памяти, а в копии для `ch12_twice_ref` есть одно, хотя замыкание
вызывается дважды? Сколько байт стека занимает структура замыкания в
`ch12_scale_dyn` на Cortex-M3? Что изменится в `ch12_map_dyn`, если
замыкание будет захватывать не `&Affine`, а `Affine` по значению, и
что — в `ch12_scale_dyn`?

</div>
