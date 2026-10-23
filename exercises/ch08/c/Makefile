# Задача 8.C: библиотека на Rust как staticlib и программа на C.
#
#   make                 собрать build/station-demo
#   make OUT=каталог     собрать в другом каталоге (так делает тест)
#
# Тест ждёт программу в $(OUT)/station-demo и архив в $(LIB).

OUT    ?= build
CARGO  ?= cargo
CFLAGS ?= -O2 -Wall -Wextra -std=c11
LIB    := $(OUT)/target/release/libstation.a

$(OUT)/station-demo: main.c station.h $(LIB)
	@echo "задача 8.C: допишите в Makefile команду сборки программы" >&2; false

# Когда пересобирать крейт, решает cargo, поэтому цель всегда устаревшая.
$(LIB): FORCE
	$(CARGO) build --release --manifest-path station/Cargo.toml --target-dir $(OUT)/target

clean:
	rm -rf $(OUT)

.PHONY: FORCE clean
FORCE:
