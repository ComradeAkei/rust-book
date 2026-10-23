#!/usr/bin/env bash
# Собирает C-пример из введения к главе 12 и сверяет с текстом:
# callback.c — реестр обработчиков с указателем на функцию и void *ctx.
# GCC и clang молчат; подписка занимает три указателя; обработчик с
# контекстом чужого типа портит соседнюю память, и это видит только ASan;
# подписка переживает контекст на стеке, ASan сообщает о
# stack-use-after-return; контекст в куче освобождает деструктор из
# подписки, и утечек нет. apply вызывает функцию по указателю в каждом
# элементе, а scale_all с -O3 векторизуется. Если есть clang, проверяется,
# что он сам специализировал apply под scale. Время печатается для
# сведения: от машины к машине оно разное.
# nested.c — вложенная функция GCC с захватом: GCC строит трамплин на
# стеке, и компоновщик помечает стек исполняемым; clang такой код
# отвергает.
# Запуск: bash code/ch12/c/run.sh. Нужен gcc, objdump и readelf.
set -u
export LC_ALL=C
cd "$(dirname "$0")"
CC=${CC:-gcc}
CLANG=${CLANG:-clang}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# expect <описание> <есть|нет> <подстрока> <текст>
expect() {
    if grep -qF -- "$3" <<<"$4"; then found=есть; else found=нет; fi
    if [ "$found" = "$2" ]; then
        echo "ok    $1"
    else
        echo "FAIL  $1: «$3» ожидалось: $2"
        echo "$4" | sed 's/^/      /'
        fail=1
    fi
}

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/callback" callback.c 2>"$tmp/gcc.log"; then
    echo "FAIL  callback.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
expect "callback.c: GCC -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/gcc.log")"
if command -v "$CLANG" >/dev/null; then
    "$CLANG" -O2 -Wall -Wextra -o "$tmp/callback-clang" callback.c 2>"$tmp/clang.log"
    expect "callback.c: clang -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/clang.log")"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/callback")
expect "callback.c: подписка — три указателя" есть "sizeof(struct handler) = 24" "$out"
expect "callback.c: контекст не того типа портит счётчик" есть "событий: 63 (ожидалось 3)" "$out"
expect "callback.c: сессия" есть "в сессии: 1" "$out"
expect "callback.c: контекст в куче" есть "среднее: 6" "$out"

"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all -o "$tmp/callback-san" callback.c
san() { ASAN_OPTIONS=detect_stack_use_after_return=1 "$tmp/callback-san" "$1" 2>&1; }
expect "callback.c под ASan: чужой контекст" есть \
    "ERROR: AddressSanitizer: global-buffer-overflow" "$(san mixup)"
expect "callback.c под ASan: место ошибки" есть "in on_average" "$(san mixup)"
expect "callback.c под ASan: контекст на мёртвом стеке" есть \
    "ERROR: AddressSanitizer: stack-use-after-return" "$(san session)"
expect "callback.c под ASan: кадр start_session" есть "in start_session" "$(san session)"
heap=$(san heap)
expect "callback.c под ASan: контекст в куче без ошибок" нет "ERROR" "$heap"
expect "callback.c под ASan: контекст в куче" есть "среднее: 6" "$heap"

"$CC" -O3 -o "$tmp/callback-o3" callback.c
dis=$(objdump -d --no-show-raw-insn -M intel "$tmp/callback-o3")
apply=$(awk '/<apply>:/,/^$/' <<<"$dis")
scale=$(awk '/<scale_all>:/,/^$/' <<<"$dis")
expect "callback.c: apply вызывает f по указателю" есть "call   r" "$apply"
expect "callback.c: apply не векторизован" нет "xmm" "$apply"
expect "callback.c: scale_all с -O3 векторизован" есть "pmullw" "$scale"
bench=$("$tmp/callback-o3" bench 200)
expect "callback.c: результаты apply и scale_all совпадают" нет "расхождение" "$bench"

if command -v "$CLANG" >/dev/null; then
    dis=$(objdump -d --no-show-raw-insn -M intel "$tmp/callback-clang")
    expect "callback.c: clang специализировал apply под scale" есть "<apply.specialized" "$dis"
fi

# Компоновщик binutils 2.39+ предупреждает об исполняемом стеке.
"$CC" -O2 -Wall -Wextra -Wtrampolines -o "$tmp/nested" nested.c 2>"$tmp/nested.log"
log=$(cat "$tmp/nested.log")
expect "nested.c: GCC строит трамплин" есть "trampoline generated for nested function 'add'" "$log"
expect "nested.c: стек исполняемый" есть "requires executable stack" "$log"
expect "nested.c: захват работает" есть "sum = 6" "$("$tmp/nested")"
expect "nested.c: GNU_STACK с правом исполнения" есть "RWE" \
    "$(readelf -lW "$tmp/nested" | grep GNU_STACK)"
if command -v "$CLANG" >/dev/null; then
    err=$("$CLANG" -c -o /dev/null nested.c 2>&1)
    expect "nested.c: clang вложенных функций не знает" есть \
        "function definition is not allowed here" "$err"
fi

for bin in callback callback-o3 ${CLANG:+callback-clang}; do
    [ -x "$tmp/$bin" ] || continue
    "$tmp/$bin" bench | sed "s/^/инфо  $bin: /"
done

exit $fail
