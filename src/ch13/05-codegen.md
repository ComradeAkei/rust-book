# 13.5. Во что компилируется цепочка

Раздел 13.2 показал, что `filter` и `map` над срезом становятся одним
циклом, как написанный руками. Здесь — случаи, где форма записи меняет
код: внешняя и внутренняя итерация, проверки границ, SIMD на x86-64 и
DSP-инструкции Cortex-M4, `chunks` против `chunks_exact`. Время на
элемент печатает пример:

```console
$ cargo run --release -p ch13 --example bench
chain, for:          0.486 нс на элемент
chain, fold:         0.071 нс на элемент
сведение, индексы:   0.081 нс на отсчёт
сведение, zip:       0.079 нс на отсчёт
сведение, AVX2:      0.036 нс на отсчёт (если процессор умеет)
пики, chunks:        0.361 нс на отсчёт
пики, chunks_exact:  0.058 нс на отсчёт
```

## for против fold: внешняя и внутренняя итерация

Сумма двух буферов через `chain`, двумя способами:

```rust
{{#include ../../code/ch13/src/codegen.rs:chain}}
```

Результат один, а скорость различается в семь раз. Цикл `for` — это
*внешняя итерация*: цикл принадлежит вызывающему, и на каждом шаге он
зовёт `Chain::next`. А `Chain::next` устроен как `RingIter::next` из
раздела 13.4: первая половина хранится как `Option`, и на каждом шаге
надо проверить, не кончилась ли она. В ассемблере это `test rdi, rdi` —
проверка, что указатель первой половины не ноль, то есть что `Option`
ещё `Some`:

```console
$ cargo asm -p ch13 --lib --simplify ch13_chain_for
ch13_chain_for:
	lea rsi, [rdi + 4*rsi]
	lea rcx, [rdx + 4*rcx]
	xor eax, eax
	test rdi, rdi
	jne .LBB59_2
	jmp .LBB59_4
.LBB59_3:
	add eax, dword ptr [rdi]
	add rdi, 4
	test rdi, rdi
	je .LBB59_4
.LBB59_2:
	cmp rdi, rsi
	jne .LBB59_3
.LBB59_4:
	cmp rdx, rcx
	je .LBB59_6
	add eax, dword ptr [rdx]
	add rdx, 4
	xor edi, edi
	test rdi, rdi
	jne .LBB59_2
	jmp .LBB59_4
.LBB59_6:
	ret
```

Один элемент за шаг, и векторизации нет: LLVM 22 не разделил цикл на
два, проверка состояния `Option` осталась в нём на каждом шаге, а цикл
с таким ветвлением он не векторизует. `fold` — *внутренняя итерация*: цикл принадлежит
итератору, и `Chain::fold` пишет его сам — свернуть первую половину её
собственным `fold`, затем вторую. Каждая половина — свёртка по срезу, и
каждая векторизована:

```console
$ cargo asm -p ch13 --lib --simplify ch13_chain_fold
ch13_chain_fold:
	test rsi, rsi
	je .LBB58_1
…
.LBB58_6:
	movdqu xmm2, xmmword ptr [rdi + r9]
	paddd xmm1, xmm2
	movdqu xmm2, xmmword ptr [rdi + r9 + 16]
	paddd xmm0, xmm2
	add r9, 32
	cmp rax, r9
	jne .LBB58_6
…
.LBB58_14:
	movdqu xmm2, xmmword ptr [rdx + rdi]
	paddd xmm0, xmm2
	movdqu xmm2, xmmword ptr [rdx + rdi + 16]
	paddd xmm1, xmm2
	add rdi, 32
	cmp rax, rdi
	jne .LBB58_14
…
```

Восемь чисел за шаг в двух регистрах `xmm`. На Cortex-M3 векторов нет,
и разница — в проверках: `ch13_chain_for` развёрнут вчетверо, но перед
каждым из четырёх сложений стоит пара `cmp`/`cmpne` — «половина ещё
`Some` и не кончилась», а у `ch13_chain_fold` развёрнутое тело —
четыре загрузки (`ldr` и `ldrd`) и четыре `add` без проверок. Тест `rustc_chain` проверяет
обе цели.

Правило отсюда простое. Через `fold` работают `sum`, `product`,
`count`, `for_each`, `max`, `min`, `last` и многие другие потребители. `for` по `chain`, `flat_map`, `flatten`,
`skip_while` и другим адаптерам с состоянием «где я» стоит заменить на
`for_each`, если тело цикла — простая свёртка. Для итератора по срезу,
`map` и `filter` над ним разницы нет: их `next` и так не проверяет
ничего лишнего. А свой итератор из двух частей должен переопределить
`fold`, как это сделал `RingIter`: задача D (раздел [13.7](07-tasks.md))
показывает, что без этого векторизации нет.

<div class="box hood">

**Под капотом.** `try_fold` — та же внутренняя итерация, но с ранним
выходом, и через неё работают `find`, `any`, `all`, `position`. У
`Chain` он тоже свой. Готовые `fold` и `try_fold` есть у
всех адаптеров стандартной библиотеки, и каждый пересылает свёртку
внутрь, к срезу: `Map::fold` зовёт `fold` внутреннего итератора с
замыканием, которое сначала применяет функцию `map`. Поэтому
`a.iter().chain(b).map(f).sum()` тоже сворачивается по половинам.

</div>

## Проверки границ

Сведение двух каналов с насыщением — три записи одного цикла:

```rust
{{#include ../../code/ch13/src/codegen.rs:mix}}
```

В `ch13_mix_index` длины `a` и `b` могут быть меньше `out.len()`, и
`a[i]` с `b[i]` обязаны проверять индекс. LLVM 22 всё равно векторизует
цикл: он вычисляет, сколько шагов заведомо пройдут проверки, —
`min(a.len(), b.len(), out.len() − 1)`, — делает их векторным циклом, а
остаток — скалярным, с проверками:

```console
$ cargo asm -p ch13 --lib --simplify ch13_mix_index
ch13_mix_index:
…
.LBB64_8:
	movdqu xmm0, xmmword ptr [rdx + 2*r10]
	movdqu xmm1, xmmword ptr [rdx + 2*r10 + 16]
	movdqu xmm2, xmmword ptr [r8 + 2*r10]
	paddsw xmm2, xmm0
	movdqu xmm0, xmmword ptr [r8 + 2*r10 + 16]
	paddsw xmm0, xmm1
	movdqu xmmword ptr [rdi + 2*r10], xmm2
	movdqu xmmword ptr [rdi + 2*r10 + 16], xmm0
	add r10, 16
	cmp rax, r10
	jne .LBB64_8
…
.LBB64_4:
	cmp r11, rax
	je .LBB64_9
	cmp r10, rax
	je .LBB64_6
	movzx ebx, word ptr [rdx + 2*rax]
…
.LBB64_9:
	lea rdx, [rip + .Lanon.8ec35787557c5e94efe5731d66c12e36.25]
	mov rdi, rcx
	mov rsi, rcx
	call qword ptr [rip + core::panicking::panic_bounds_check@GOTPCREL]
…
```

`paddsw` — сложение восьми 16-битных чисел с насыщением, ровно
`saturating_add`. В скалярном хвосте две проверки — `a` и `b` — и два
вызова `panic_bounds_check`. Паника при этом случится в том же месте,
что и в цикле без векторизации: порядок записей в `out` до неё тот же.

`ch13_mix_assert` проверяет длины один раз, перед циклом. После
`assert!` компилятор знает, что `i < n ≤ a.len()`, и проверок в цикле
нет: вызов паники один, у `assert!`. `ch13_mix_zip` не проверяет вовсе:
`zip` идёт до конца самого короткого из трёх, и индексов нет. Обе
функции — векторный цикл по 16 отсчётов, векторный по 4 и скалярный
хвост, и обе без `panic_bounds_check` (тест `rustc_bounds_x86`).

На x86-64 время у трёх вариантов почти одинаковое: работает векторный
цикл, а проверки остались в хвосте. На Cortex-M разница видна в самом
цикле. У M3 `ch13_mix_index` не развёрнут, и в каждой итерации — две
проверки и одна `ssat` (насыщение до 16 бит):

```console
$ cargo asm -p ch13 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch13_mix_index
ch13_mix_index:
	push {r4, r5, r6, r7, lr}
	add r7, sp, #12
	str r8, [sp, #-4]!
	cbz r1, .LBB35_5
	ldrd lr, r12, [r7, #8]
	movs r4, #0
.LBB35_2:
	cmp r3, r4
	beq .LBB35_6
	cmp r12, r4
	beq .LBB35_7
	ldrsh.w r8, [lr, r4, lsl #1]
	adds r5, r4, #1
	ldrsh.w r6, [r2, r4, lsl #1]
	cmp r1, r5
	add r6, r8
	ssat r6, #16, r6
	strh.w r6, [r0, r4, lsl #1]
	mov r4, r5
	bne .LBB35_2
…
```

А `ch13_mix_zip` развёрнут вчетверо: восемь `ldrsh`, четыре `ssat` и
четыре `strh` за итерацию, и ни одного сравнения, кроме счётчика. У M0
нет и `ssat`: насыщение — два сравнения с константами 32767 и −32768 на
каждый отсчёт, и проверки индексов к ним добавляются.

<div class="box trick">

**Трюк.** `assert!` на длины перед циклом по индексам — дешёвый способ
убрать проверки из цикла, когда индексная запись понятнее итераторной
(например, `x[i]` и `x[i + 3]` в одном выражении). Проверка одна,
паника понятная — «assertion failed: a.len() >= n && b.len() >= n», а
не «index out of bounds» на середине записи. Срез `let a = &a[..n];`
делает то же: после него `a.len() == n` известна компилятору.

</div>

## AVX2 и выбор во время работы

Тот же цикл можно собрать под AVX2 — регистры `ymm` вдвое шире `xmm`, —
не собирая под AVX2 всю программу:

```rust
{{#include ../../code/ch13/src/codegen.rs:avx2}}
```

`#[target_feature(enable = "avx2")]` разрешает компилятору AVX2 только
в этой функции. Объявить её безопасной можно с Rust 1.86, а вызвать из
кода без того же атрибута — только в `unsafe`: обещать, что процессор
умеет AVX2, должен вызывающий. `is_x86_feature_detected!` проверяет
CPUID один раз и кэширует ответ. В ассемблере — те же два цикла, но
шире и развёрнуты вчетверо, 64 отсчёта за итерацию:

```console
$ cargo asm -p ch13 --lib --simplify ch13_mix_avx2
ch13_mix_avx2:
…
.LBB63_7:
	vmovdqu ymm0, ymmword ptr [rdx + r10]
	vmovdqu ymm1, ymmword ptr [rdx + r10 + 32]
	vmovdqu ymm2, ymmword ptr [rdx + r10 + 64]
	vmovdqu ymm3, ymmword ptr [rdx + r10 + 96]
	vpaddsw ymm0, ymm0, ymmword ptr [r8 + r10]
	vpaddsw ymm1, ymm1, ymmword ptr [r8 + r10 + 32]
	vpaddsw ymm2, ymm2, ymmword ptr [r8 + r10 + 64]
	vpaddsw ymm3, ymm3, ymmword ptr [r8 + r10 + 96]
	vmovdqu ymmword ptr [rdi + r10], ymm0
	vmovdqu ymmword ptr [rdi + r10 + 32], ymm1
	vmovdqu ymmword ptr [rdi + r10 + 64], ymm2
	vmovdqu ymmword ptr [rdi + r10 + 96], ymm3
	sub r10, -128
	cmp r9, r10
	jne .LBB63_7
…
	vzeroupper
	ret
```

`vzeroupper` в конце обнуляет верхние половины `ymm`: без него код SSE,
который выполнится после функции, платил бы за переход между
состояниями регистров. На машине из бенчмарка AVX2-вариант вдвое
быстрее: 0,036 нс на отсчёт против 0,079.

<div class="box ref">

**Справка: SSE2 и AVX2.** SSE2 входит в базовый набор x86-64, и
`x86_64-unknown-linux-gnu` по умолчанию собирается только с ним:
регистры `xmm` по 128 бит, восемь `i16` за инструкцию. AVX2 — расширение
2013 года (Haswell), регистры `ymm` по 256 бит. Собрать всю программу
под процессор сборщика — `RUSTFLAGS="-C target-cpu=native"`, но такая
программа упадёт с «недопустимой инструкцией» на процессоре без AVX2.
Выбор во время работы, как в `mix`, даёт один двоичный файл для всех.

</div>

## Cortex-M4: DSP-инструкции

У Cortex-M4 нет векторных регистров, но есть DSP-расширение:
инструкции, которые работают с двумя 16-битными половинами 32-битного
регистра. LLVM пользуется ими частично. В `ch13_mix_zip` для
`thumbv7em-none-eabihf` вместо `add` и `ssat` стоит одна `qadd16` —
сложение двух пар половин с насыщением, — но отсчёты загружаются по
одному `ldrsh`, и из двух сложений полезно одно. Скалярное
произведение — отвод КИХ-фильтра — получается лучше:

```rust
{{#include ../../code/ch13/src/codegen.rs:dot}}
```

```console
$ cargo asm -p ch13 --lib --simplify --no-default-features --target thumbv7em-none-eabihf ch13_dot_i16
ch13_dot_i16:
…
.LBB32_3:
	ldr.w r9, [r0, r4]
	adds r1, r2, r4
	ldr r6, [r2, r4]
	adds r5, r0, r4
	ldr r1, [r1, #4]
	adds r4, #8
	ldr r5, [r5, #4]
	adds r3, #4
	smlad r6, r6, r9, r8
	cmp lr, r4
	smlad r8, r1, r5, r6
	bne .LBB32_3
…
```

`ldr` читает два отсчёта сразу, а `smlad` умножает обе пары половин и
прибавляет оба произведения к сумме: четыре умножения с накоплением за
две инструкции. На M3 того же нет: там четыре `mla` на четыре отсчёта,
каждый со своим `ldrsh`. На x86-64 цикл векторизован через `pmaddwd`:
LLVM перемежает отсчёты нулями и пользуется ею как умножением четырёх
пар 16×16 → 32 бита за инструкцию. Тест `rustc_bounds_thumb` проверяет
`qadd16` и `smlad` на M4F, `ssat` и `mla` без `smlad` на M3.

## chunks против chunks_exact

Пиковое значение каждого блока из 16 отсчётов:

```rust
{{#include ../../code/ch13/src/codegen.rs:peaks}}
```

У `chunks(16)` последний кусок может быть короче, поэтому длина
внутреннего `max` неизвестна, и для каждого блока LLVM строит цикл с
векторной частью, узкой векторной частью и скалярным хвостом — 129
инструкций. У `chunks_exact(16)` каждый кусок ровно из 16
отсчётов, и внутренний цикл исчезает:

```console
$ cargo asm -p ch13 --lib --simplify ch13_peaks_exact
ch13_peaks_exact:
	shr rsi, 4
	cmp rsi, rcx
	cmovae rsi, rcx
…
.LBB68_4:
	movdqu xmm0, xmmword ptr [r8 - 32]
	movdqu xmm1, xmmword ptr [r8 - 16]
	pmaxsw xmm1, xmm0
	pshufd xmm0, xmm1, 238
	pmaxsw xmm0, xmm1
	pshufd xmm1, xmm0, 85
	pmaxsw xmm1, xmm0
	movdqa xmm0, xmm1
	psrld xmm0, 16
	pmaxsw xmm0, xmm1
	movd r9d, xmm0
	mov word ptr [rdx + 2*rax], r9w
…
```

Блок — две загрузки по 16 байт, `pmaxsw` попарно, и три шага свёртки
внутри регистра: `pshufd` и `psrld` сдвигают половины, `pmaxsw` берёт
максимум. Число блоков `shr rsi, 4` — `x.len() / 16` — вычислено один
раз, и неполный хвост не обрабатывается: его отдаёт `remainder()`, если
он нужен. Разница по времени — шесть раз, 0,058 нс на отсчёт против
0,361. На M3 `chunks_exact` тоже убирает внутренний цикл: блок
разворачивается в загрузки `ldrsh` и цепочку сравнений с условным
`mov`.

<div class="box check">

**Проверь себя.** Почему `for x in a.iter().chain(b)` медленнее, чем
`for x in a { … } for x in b { … }`, а `a.iter().chain(b).sum()` — нет?
Будет ли векторизован `ch13_mix_index`, если `out`, `a` и `b` —
массивы `[i16; 64]` по ссылке, и останутся ли в нём проверки? Почему в
`ch13_peaks_exact` нет вызова паники, хотя `max()` возвращает
`Option`?

</div>
