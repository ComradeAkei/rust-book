#!/usr/bin/env bash
# Раздел 30.5: оптимизация по профилю (PGO) программы inlining.
#
# 1. Сборка с -Cprofile-generate: программа пишет счётчики переходов в
#    файлы .profraw.
# 2. Запуск на типичной нагрузке (в inlining — миллион вызовов check_*
#    со значениями в диапазоне).
# 3. llvm-profdata merge сводит сырые профили в один .profdata.
# 4. Сборка с -Cprofile-use: LLVM раскладывает код по профилю. Профиль
#    собран со всей программы, а check_plain — функция библиотеки ch30,
#    поэтому ассемблер снимается с библиотеки.
#
# Скрипт печатает тело check_plain без профиля и с профилем и проверяет,
# что с профилем горячий путь идёт подряд (переход `ja` к редкой ветке),
# а без профиля — нет (`jbe` к горячей). llvm-profdata той же версии
# LLVM, что у rustc, даёт компонент rustup: rustup component add llvm-tools
set -euo pipefail
cd "$(dirname "$0")/../../.."

host=$(rustc -vV | sed -n 's/^host: //p')
bin="$(rustc --print sysroot)/lib/rustlib/$host/bin"
if [ ! -x "$bin/llvm-profdata" ]; then
  echo "нет $bin/llvm-profdata: rustup component add llvm-tools" >&2
  exit 1
fi

dir="$PWD/target/ch30-pgo"
rm -rf "$dir"
mkdir -p "$dir"
# Единицы кодогенерации (одна) и отладочная информация — одинаковые во
# всех трёх сборках: профиль привязан к графу переходов каждой функции, и
# с другими настройками LLVM отбросил бы счётчики части функций
# («function control flow change detected (hash mismatch)»).
asm=(-- --emit asm -C llvm-args=-x86-asm-syntax=intel)

# Тело функции из файла .s: от метки до .Lfunc_end, без директив.
body() {
  awk -v name="$2:" '
    /^_R/ && substr($0, length($0) - length(name) + 1) == name { on = 1; next }
    on && /^\.Lfunc_end/ { exit }
    on && $0 !~ /^[ \t]*[.#]/ && $0 !~ /^[ \t]*$/ { sub(/^[ \t]+/, ""); gsub(/\t/, " "); print }
    on && /^\.LBB/ { print }
  ' "$1"
}

# ANCHOR: steps
RUSTFLAGS="-Cprofile-generate=$dir/raw -Ccodegen-units=1" \
  cargo build -q --release -p ch30 --bin inlining --target-dir "$dir/gen"
"$dir/gen/release/inlining" > "$dir/run.txt"
"$bin/llvm-profdata" merge -o "$dir/ch30.profdata" "$dir/raw"
RUSTFLAGS="-Cprofile-use=$dir/ch30.profdata -Ccodegen-units=1" \
  cargo rustc -q --release -p ch30 --lib --target-dir "$dir/use" "${asm[@]}"
# ANCHOR_END: steps
RUSTFLAGS="-Ccodegen-units=1" \
  cargo rustc -q --release -p ch30 --lib --target-dir "$dir/plain" "${asm[@]}"

plain=$(ls "$dir"/plain/release/deps/ch30-*.s)
pgo=$(ls "$dir"/use/release/deps/ch30-*.s)
echo "профиль: $(ls "$dir"/raw | wc -l) файл(а) .profraw, $(wc -c < "$dir/ch30.profdata") байт .profdata"
echo "--- check_plain без профиля"
body "$plain" 11check_plain | tee "$dir/plain.txt"
echo "--- check_plain с профилем"
body "$pgo" 11check_plain | tee "$dir/pgo.txt"

grep -q '^jbe ' "$dir/plain.txt" || { echo "без профиля ждали jbe" >&2; exit 1; }
grep -q '^ja ' "$dir/pgo.txt" || { echo "с профилем ждали ja" >&2; exit 1; }
echo "ok"
