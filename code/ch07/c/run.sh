#!/usr/bin/env bash
# Собирает C-примеры из введения к главе 7 и сверяет с текстом:
# - errno.c: сообщение об ошибке называет не ту причину, потому что errno
#   перезаписал вызов между open и strerror, а анализатор GCC молчит;
# - unchecked.c: забытый код возврата не замечает никто, пока функция не
#   помечена warn_unused_result; GCC предупреждает и после (void), clang нет;
# - longjmp.c: longjmp перепрыгивает через free и unlock — LeakSanitizer
#   находит утечку, а шина остаётся занятой навсегда.
# Запуск: bash code/ch07/c/run.sh. Нужен gcc с libasan; если есть clang,
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

# build <выход> <флаги и файлы...>: собирает и при ошибке печатает журнал.
build() {
    local out="$1"
    shift
    if ! "$CC" -o "$tmp/$out" "$@" 2>"$tmp/$out.log"; then
        echo "FAIL  не собирается: $*"
        sed 's/^/      /' "$tmp/$out.log"
        fail=1
        return 1
    fi
}

# errno.c: open вернул EISDIR, но до strerror errno успел перезаписать fopen.
if build errno -O2 -Wall -Wextra -fanalyzer errno.c; then
    expect "errno.c: GCC -Wall -Wextra -fanalyzer молчит" нет "warning" "$(cat "$tmp/errno.log")"
    out=$("$tmp/errno" 2>&1)
    expect "errno.c: open завершился ошибкой" есть "open /tmp: " "$out"
    expect "errno.c: причина названа неверно" есть "No such file or directory" "$out"
    expect "errno.c: настоящей причины в сообщении нет" нет "Is a directory" "$out"
fi

# unchecked.c: без атрибута компилятор не видит забытую проверку.
if build unchecked -O2 -Wall -Wextra -fanalyzer unchecked.c sensor.c; then
    expect "unchecked.c: без атрибута GCC молчит" нет "warning" "$(cat "$tmp/unchecked.log")"
fi
if build must -O2 -Wall -Wextra -DMUST_USE unchecked.c sensor.c; then
    expect "unchecked.c: warn_unused_result замечен GCC" есть \
        "ignoring return value of 'read_sensor' declared with attribute 'warn_unused_result'" \
        "$(cat "$tmp/must.log")"
fi
if build voidcast -O2 -Wall -Wextra -DMUST_USE -DVOID_CAST unchecked.c sensor.c; then
    expect "unchecked.c: GCC предупреждает и после (void)" есть "10 |     (void)read_sensor" \
        "$(cat "$tmp/voidcast.log")"
fi
if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" -O2 -Wall -Wextra -DMUST_USE -DVOID_CAST -c -o "$tmp/u.o" unchecked.c 2>&1)
    expect "unchecked.c: clang предупреждает о строке 8" есть "unchecked.c:8:5: warning" "$w"
    expect "unchecked.c: clang принимает (void)" нет "unchecked.c:10" "$w"
else
    echo "skip  clang: не найден"
fi

# longjmp.c: копия сообщения утекает, мьютекс шины остаётся захваченным.
if build longjmp -O2 -Wall -Wextra -fanalyzer longjmp.c; then
    expect "longjmp.c: анализатор GCC молчит" нет "warning" "$(cat "$tmp/longjmp.log")"
    out=$("$tmp/longjmp")
    expect "longjmp.c: первое сообщение принято" есть "A1: принято" "$out"
    expect "longjmp.c: второе сообщение с ошибкой" есть "B2: ошибка разбора" "$out"
    expect "longjmp.c: шина осталась занятой" есть "A3: шина занята" "$out"
fi
if build asan -O1 -g -fsanitize=address longjmp.c; then
    out=$("$tmp/asan" 2>&1)
    expect "longjmp.c: LeakSanitizer нашёл утечку" есть \
        "Direct leak of 64 byte(s) in 1 object(s) allocated from" "$out"
fi

exit $fail
