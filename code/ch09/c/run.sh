#!/usr/bin/env bash
# Собирает C-пример из введения к главе 9 и сверяет с текстом:
# ops.c — полиморфизм через таблицу указателей на функции и void *.
# Ни GCC, ни clang, ни анализатор GCC не замечают ни перепутанного
# контекста, ни незаполненного поля таблицы; программа печатает мусор и
# падает на вызове по нулевому указателю. poll_twice вызывает read
# косвенно и перечитывает таблицу перед вторым вызовом.
# Запуск: bash code/ch09/c/run.sh. Нужен gcc и objdump; если есть clang,
# проверяется и он.
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

if ! "$CC" -O2 -Wall -Wextra -fanalyzer -o "$tmp/ops" ops.c 2>"$tmp/gcc.log"; then
    echo "FAIL  ops.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
expect "ops.c: GCC -Wall -Wextra -fanalyzer молчит" нет "warning" "$(cat "$tmp/gcc.log")"
if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" -O2 -Wall -Wextra -c -o "$tmp/ops-clang.o" ops.c 2>&1)
    expect "ops.c: clang -Wall -Wextra молчит" нет "warning" "$w"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/ops" 2>&1)
status=$?
expect "ops.c: термодатчик" есть "0: poll_twice = 4300" "$out"
expect "ops.c: счётчик" есть "1: poll_twice = 6" "$out"
expect "ops.c: чужой контекст читается как свой" есть "2: poll_twice = 20" "$out"
expect "ops.c: имя первого датчика" есть "0: термодатчик" "$out"
expect "ops.c: до второго имени не доходит" нет "1: " "$(grep -v poll_twice <<<"$out")"
if [ "$status" -eq 139 ]; then
    echo "ok    ops.c: SIGSEGV на вызове по нулевому указателю"
else
    echo "FAIL  ops.c: ждали SIGSEGV (139), а код $status"
    fail=1
fi

asm=$(objdump -d --no-show-raw-insn -M intel "$tmp/ops" | awk '/<poll_twice>:/,/^$/')
expect "ops.c: poll_twice вызывает read косвенно" есть "call   QWORD PTR [rax]" "$asm"
n=$(grep -c "mov    rax,QWORD PTR \[rbx\]" <<<"$asm")
if [ "$n" -eq 2 ]; then
    echo "ok    ops.c: таблица перечитывается перед вторым вызовом"
else
    echo "FAIL  ops.c: ждали два чтения s->ops, а их $n"
    echo "$asm" | sed 's/^/      /'
    fail=1
fi

exit $fail
