# Задача 8.C: библиотека на Rust как staticlib и программа на C.
#
#   make                 собрать build/station-demo
#   make OUT=каталог     собрать в другом каталоге (так делает тест)

OUT    ?= build
CARGO  ?= cargo
CFLAGS ?= -O2 -Wall -Wextra -std=c11
LIB    := $(OUT)/target/release/libstation.a

# Библиотеки системы, которые нужны std внутри libstation.a. Список
# печатает rustc: cargo rustc --release -- --print native-static-libs.
# Порядок важен: линковщик ищет символы только в библиотеках правее.
LDLIBS := -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc

$(OUT)/station-demo: main.c station.h $(LIB)
	$(CC) $(CFLAGS) -o $@ main.c $(LIB) $(LDLIBS)

# Когда пересобирать крейт, решает cargo, поэтому цель всегда устаревшая.
$(LIB): FORCE
	$(CARGO) build --release --manifest-path station/Cargo.toml --target-dir $(OUT)/target

clean:
	rm -rf $(OUT)

.PHONY: FORCE clean
FORCE:
