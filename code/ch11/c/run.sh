#!/usr/bin/env bash
# Собирает C-пример из введения к главе 11 и сверяет с текстом:
# fops.c — таблица методов по образцу struct file_operations. GCC и clang
# молчат; указатель на таблицу занимает место в каждом объекте;
# необязательный метод проверяет вызывающий; ASan и UBSan ошибок не
# находят; poll вызывает read по адресу из таблицы, а с retpoline — через
# переходник с pause и lfence. Если есть valgrind, его модель предсказателя
# показывает промахи косвенных переходов по порядку и вперемешку. Время
# вызова печатается для сведения: от машины к машине оно разное.
# Запуск: bash code/ch11/c/run.sh. Нужен gcc и objdump; если есть clang,
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

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/fops" fops.c 2>"$tmp/gcc.log"; then
    echo "FAIL  fops.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
expect "fops.c: GCC -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/gcc.log")"
if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" -O2 -Wall -Wextra -c -o "$tmp/fops-clang.o" fops.c 2>&1)
    expect "fops.c: clang -Wall -Wextra молчит" нет "warning" "$w"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/fops")
expect "fops.c: указатель на таблицу в каждом объекте" есть \
    "sizeof(struct thermo) = 24, sizeof(struct counter) = 24" "$out"
expect "fops.c: чтение через таблицу" есть "thermo: 6, counter: 999" "$out"
expect "fops.c: write без метода — -EINVAL" есть "write: -22, 0" "$out"

"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all -o "$tmp/fops-san" fops.c
for args in "" "bench shuffled 10"; do
    # shellcheck disable=SC2086
    if "$tmp/fops-san" $args >/dev/null 2>"$tmp/san.log"; then
        echo "ok    fops.c: ASan и UBSan молчат (${args:-демо})"
    else
        echo "FAIL  fops.c: санитайзеры нашли ошибку (${args:-демо})"
        sed 's/^/      /' "$tmp/san.log"
        fail=1
    fi
done

sorted=$("$tmp/fops" bench sorted 1000)
shuffled=$("$tmp/fops" bench shuffled 1000)
expect "fops.c: по порядку и вперемешку — та же сумма" есть "сумма 1154250000" "$sorted"
expect "fops.c: перестановка не меняет суммы" есть "сумма 1154250000" "$shuffled"

dis=$(objdump -d --no-show-raw-insn -M intel "$tmp/fops")
poll=$(awk '/<poll[^>]*>:/,/^$/' <<<"$dis")
expect "fops.c: poll вызывает read по адресу из таблицы" есть "call   QWORD PTR [rax]" "$poll"

# Ubuntu собирает GCC с -fcf-protection по умолчанию, а с ним
# -mindirect-branch несовместим.
"$CC" -O2 -fcf-protection=none -mindirect-branch=thunk -o "$tmp/fops-ret" fops.c
dis=$(objdump -d --no-show-raw-insn -M intel "$tmp/fops-ret")
poll=$(awk '/<poll[^>]*>:/,/^$/' <<<"$dis")
thunk=$(awk '/<__x86_indirect_thunk_rax>:/,/^$/' <<<"$dis")
expect "fops.c с retpoline: вызов через переходник" есть "call" "$(grep '__x86_indirect_thunk_rax' <<<"$poll")"
expect "fops.c с retpoline: косвенного call нет" нет "call   QWORD PTR" "$poll"
expect "fops.c с retpoline: ловушка pause" есть "pause" "$thunk"
expect "fops.c с retpoline: ловушка lfence" есть "lfence" "$thunk"

# Модель предсказателя valgrind детерминирована: по порядку промахи только
# на двух сменах типа за проход, вперемешку — около половины вызовов.
if command -v valgrind >/dev/null; then
    for order in sorted shuffled; do
        rate=$(valgrind --tool=cachegrind --cache-sim=no --branch-sim=yes \
            --cachegrind-out-file=/dev/null "$tmp/fops" bench $order 1000 2>&1 |
            sed -n 's/.*Mispred rate:.*+ *\([0-9.]*\)% *)/\1/p')
        echo "инфо  valgrind, $order: промахи косвенных переходов $rate %"
        if [ "$order" = sorted ]; then
            ok=$(awk -v r="${rate:-100}" 'BEGIN { print (r < 1) }')
        else
            ok=$(awk -v r="${rate:-0}" 'BEGIN { print (r > 40 && r < 60) }')
        fi
        if [ "$ok" = 1 ]; then
            echo "ok    fops.c: модель предсказателя, $order"
        else
            echo "FAIL  fops.c: промахи $order: «$rate»"
            fail=1
        fi
    done
else
    echo "skip  valgrind: не найден"
fi

for bin in fops fops-ret; do
    for order in sorted shuffled; do
        echo "инфо  $bin: $("$tmp/$bin" bench $order 20000)"
    done
done

exit $fail
