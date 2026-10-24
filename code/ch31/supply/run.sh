#!/usr/bin/env bash
# Раздел 31.9: цепочка поставок и исходы из текста главы.
# 1. Политика учебника (deny.toml в корне) проходит на пяти рабочих
#    пространствах — корень, прошивки и цели фаззинга главы, задач и
#    решений, — а без неё те же зависимости не проходят: лицензии не
#    разрешены, bare-metal заброшен.
# 2. Политика останавливает зависимость со сценарием сборки, которого нет
#    в списке, а процедурный макрос пропускает. Оба запускают свой код уже
#    при cargo check, но не при cargo metadata.
# 3. cargo audit: в Cargo.lock учебника уязвимостей нет, есть заброшенный
#    bare-metal; в Cargo.lock со старым smallvec — переполнение буфера.
#    Уязвимый крейт при этом не скачивается и не собирается: cargo audit
#    читает только Cargo.lock.
# 4. cargo vet: после init всё в исключениях, новая зависимость — 16 310
#    строк к прочтению, после certify проверка проходит.
# Запуск: bash code/ch31/supply/run.sh. Нужны cargo-deny 0.20.2,
# cargo-audit 0.22.2, cargo-vet 0.10.3 и сеть: базы уязвимостей и
# сведения crates.io.
set -u
export LC_ALL=C CARGO_TERM_COLOR=never
cd "$(dirname "$0")/../../.." || exit 2
root=$PWD
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# check <что> <код> <ожидаемый код> <файл> <подстрока>...
check() {
    local what=$1 got=$2 want=$3 log=$4 n missing=
    shift 4
    for n in "$@"; do
        grep -qF -- "$n" "$log" || missing+=" «$n»"
    done
    if [ "$got" -eq "$want" ] && [ -z "$missing" ]; then
        echo "ok    $what"
    else
        echo "FAIL  $what: код $got (ожидался $want)${missing:+, нет$missing}"
        tail -n 30 "$log" | sed 's/^/      /'
        fail=1
    fi
}

# Код выхода cargo deny — битовая маска проваленных проверок:
# advisories 1, bans 2, licenses 4, sources 8.

# --- 1. Политика учебника
for m in Cargo.toml firmware/Cargo.toml code/ch31/fuzz/Cargo.toml \
    exercises/ch31/fuzz/Cargo.toml solutions/ch31/fuzz/Cargo.toml; do
    cargo deny --manifest-path "$m" --config deny.toml check >"$tmp/deny.log" 2>&1
    check "cargo deny: $m" $? 0 "$tmp/deny.log" "advisories ok, bans ok, licenses ok, sources ok"
done
: >"$tmp/empty.toml"
cargo deny --config "$tmp/empty.toml" check >"$tmp/default.log" 2>&1
code=$?
check "cargo deny без политики: лицензий не разрешено $(grep -c '^error\[rejected\]' "$tmp/default.log")" \
    "$code" 5 "$tmp/default.log" \
    "advisories FAILED, bans ok, licenses FAILED, sources ok" \
    "error[unmaintained]: bare-metal is deprecated" "ID: RUSTSEC-2026-0110"

# --- 2. Код, который запускается при сборке
cp -r code/ch31/supply/build "$tmp/build"
app=$tmp/build/app
cargo deny --manifest-path "$app/Cargo.toml" --config deny.toml check bans >"$tmp/bans.log" 2>&1
code=$?
check "cargo deny: сценарий сборки не из списка" "$code" 2 "$tmp/bans.log" \
    "error[build-script-not-allowed]: crate 'ch31-supply-dep = 0.1.0' has a build script but is not allowed to have one"
if grep -qF "ch31-supply-mac = 0.1.0' has" "$tmp/bans.log"; then
    echo "FAIL  cargo deny остановил процедурный макрос, а в тексте — нет"
    fail=1
else
    echo "ok    процедурный макрос политика сборочных сценариев не видит"
fi
cargo metadata --manifest-path "$app/Cargo.toml" --format-version 1 >"$tmp/meta.json" 2>"$tmp/meta.log"
code=$?
if [ "$code" -eq 0 ] && ! ls "$tmp/build/build-rs-ran" "$tmp/build/proc-macro-ran" >"$tmp/ls" 2>&1; then
    echo "ok    cargo metadata чужого кода не запускает"
else
    echo "FAIL  cargo metadata: код $code, следы: $(cat "$tmp/ls")"
    fail=1
fi
cargo check -q --manifest-path "$app/Cargo.toml" >"$tmp/check.log" 2>&1
code=$?
cat "$tmp/build/build-rs-ran" "$tmp/build/proc-macro-ran" >>"$tmp/check.log" 2>&1
check "cargo check: $(cat "$tmp/build/build-rs-ran" 2>&1); $(cat "$tmp/build/proc-macro-ran" 2>&1)" \
    "$code" 0 "$tmp/check.log" "сценарий сборки, uid $(id -u)" "процедурный макрос"

# --- 3. cargo audit
cargo audit >"$tmp/audit.log" 2>&1
check "cargo audit: Cargo.lock учебника" $? 0 "$tmp/audit.log" \
    "ID:        RUSTSEC-2026-0110" "Warning:   unmaintained"
mkdir "$tmp/old"
# ANCHOR: old_lock
cat >"$tmp/old/Cargo.lock" <<'LOCK'
# Такой Cargo.lock cargo записал бы для зависимости smallvec = "=1.6.0".
version = 4

[[package]]
name = "old"
version = "0.1.0"
dependencies = [
 "smallvec",
]

[[package]]
name = "smallvec"
version = "1.6.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
LOCK
# ANCHOR_END: old_lock
cargo audit -f "$tmp/old/Cargo.lock" >"$tmp/old.log" 2>&1
check "cargo audit: smallvec 1.6.0" $? 1 "$tmp/old.log" \
    "ID:        RUSTSEC-2021-0003" "Title:     Buffer overflow in SmallVec::insert_many" \
    "Solution:  Upgrade to >=0.6.14, <1.0.0 OR >=1.6.1" "error: 1 vulnerability found!"

# --- 4. cargo vet
cp -r code/ch31/supply/vet "$tmp/vet"
cd "$tmp/vet" || exit 2
{ cargo generate-lockfile -q && cargo vet init && cargo vet; } >"$tmp/vet.log" 2>&1
check "cargo vet init: всё в исключениях" $? 0 "$tmp/vet.log" "Vetting Succeeded (1 exempted)"
cargo add -q memchr@=2.8.3 --no-default-features >"$tmp/add.log" 2>&1 || cat "$tmp/add.log"
cargo vet >"$tmp/vet.log" 2>&1
check "cargo vet: новая зависимость" $? 255 "$tmp/vet.log" \
    'memchr:2.8.3 missing ["safe-to-deploy"]' "cargo vet inspect memchr 2.8.3" \
    "estimated audit backlog: 16310 lines"
{ cargo vet certify memchr 2.8.3 --criteria safe-to-deploy --who "Читатель главы 31" \
    --notes "пример записи: что прочитано и что проверено" --accept-all && cargo vet; } \
    >"$tmp/vet.log" 2>&1
check "cargo vet certify" $? 0 "$tmp/vet.log" "Vetting Succeeded (1 fully audited, 1 exempted)"
cd "$root" || exit 2

if [ "$fail" -ne 0 ]; then
    echo "цепочка поставок главы 31: есть расхождения"
    exit 1
fi
echo "цепочка поставок главы 31: исходы совпали с текстом"
