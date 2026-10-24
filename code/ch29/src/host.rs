//! Обвязка на машине разработчика: собранное ядро, сборка модулей,
//! initramfs и запуск QEMU.
//!
//! Ядро собирает `code/ch29/kernel/build.sh` — в `target/ch29-kernel`
//! или туда, куда укажут переменные `CH29_KERNEL` (обычное ядро) и
//! `CH29_KERNEL_DEBUG` (ядро с KASAN, kmemleak и lockdep). Тесты, которым
//! ядра нет, пропускаются с сообщением, если не задана `CH29_REQUIRE_KERNEL`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Какое ядро нужно опыту.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    /// tinyconfig + `kernel/ch29.config`.
    Plain,
    /// То же + `kernel/ch29-debug.config`: KASAN, kmemleak, lockdep,
    /// проверка сна в атомарном контексте.
    Debug,
}

/// Каталог сборки ядра (`make O=…`).
#[derive(Debug, Clone)]
pub struct Kernel {
    pub build: PathBuf,
    pub flavor: Flavor,
}

/// Корень репозитория.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

impl Kernel {
    /// Ищет собранное ядро: переменная окружения, потом `target/ch29-kernel`.
    pub fn find(flavor: Flavor) -> Option<Kernel> {
        let (var, dir) = match flavor {
            Flavor::Plain => ("CH29_KERNEL", "build"),
            Flavor::Debug => ("CH29_KERNEL_DEBUG", "build-debug"),
        };
        let build = match std::env::var_os(var) {
            Some(p) => PathBuf::from(p),
            None => repo_root().join("target/ch29-kernel").join(dir),
        };
        build.join("arch/x86/boot/bzImage").is_file().then_some(Kernel { build, flavor })
    }

    /// Как `find`, но печатает, почему опыт пропущен. С переменной
    /// `CH29_REQUIRE_KERNEL` (так в CI) пропуска нет: без ядра тест падает.
    pub fn or_skip(flavor: Flavor) -> Option<Kernel> {
        let k = Kernel::find(flavor);
        if k.is_none() {
            assert!(
                std::env::var_os("CH29_REQUIRE_KERNEL").is_none(),
                "нет собранного ядра ({flavor:?}), а CH29_REQUIRE_KERNEL задана"
            );
            eprintln!(
                "пропуск: нет собранного ядра ({flavor:?}); соберите его: bash code/ch29/kernel/build.sh"
            );
        }
        k
    }

    pub fn bzimage(&self) -> PathBuf {
        self.build.join("arch/x86/boot/bzImage")
    }

    /// Значение из `include/config/auto.conf` — итоговой конфигурации.
    pub fn config(&self, name: &str) -> Option<String> {
        let auto = fs::read_to_string(self.build.join("include/config/auto.conf")).ok()?;
        let prefix = format!("{name}=");
        auto.lines().find_map(|l| l.strip_prefix(&prefix)).map(|v| v.trim_matches('"').to_string())
    }

    /// Версия rustc, которой собрано ядро, — `1.97.0`. Модули обязаны
    /// собираться ею же: метаданные крейта `kernel` другой версии rustc не
    /// прочитает.
    pub fn rustc_version(&self) -> String {
        let text = self.config("CONFIG_RUSTC_VERSION_TEXT").expect("ядро без CONFIG_RUST");
        text.split_whitespace().nth(1).expect("rustc X.Y.Z").to_string()
    }

    // ANCHOR: make
    /// `make M=src MO=out` — внешние модули из каталога `src` с объектами в
    /// `out`. Исходники остаются чистыми. `Err` — журнал сборки.
    pub fn make_modules(&self, src: &Path, out: &Path) -> Result<String, String> {
        self.make_modules_with(src, out, &[])
    }

    /// То же с добавочными аргументами make: `CLIPPY=1` — вместо rustc
    /// clippy-driver с линтами ядра (`// SAFETY:` у каждого `unsafe`,
    /// API не новее минимальной версии rustc ядра).
    pub fn make_modules_with(
        &self,
        src: &Path,
        out: &Path,
        extra: &[&str],
    ) -> Result<String, String> {
        fs::create_dir_all(out).map_err(|e| e.to_string())?;
        let jobs = std::thread::available_parallelism().map_or(1, |n| n.get());
        let output = Command::new("make")
            .arg("-C")
            .arg(&self.build)
            .arg(format!("M={}", src.canonicalize().map_err(|e| e.to_string())?.display()))
            .arg(format!("MO={}", out.canonicalize().map_err(|e| e.to_string())?.display()))
            .args(["LLVM=1", &format!("-j{jobs}")])
            .args(extra)
            .env("RUSTUP_TOOLCHAIN", self.rustc_version())
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            // Под `cargo test` clippy-driver увидел бы rust-version книги и
            // предупредил, что она расходится с .clippy.toml ядра (1.78).
            .env_remove("CARGO_PKG_RUST_VERSION")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("make: {e}"))?;
        let log = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        if output.status.success() { Ok(log) } else { Err(log) }
    }
    // ANCHOR_END: make

    /// Утилита ядра, которая собирает cpio по списку: `usr/gen_init_cpio`.
    pub fn gen_init_cpio(&self) -> PathBuf {
        self.build.join("usr/gen_init_cpio")
    }
}

/// Программа гостя: `cargo build --release` статически, с crt-static, в
/// отдельный каталог target — у вложенной сборки свои RUSTFLAGS.
pub fn guest_binary(package: &str, bin: &str, target_dir: &Path) -> PathBuf {
    let out = Command::new(env!("CARGO"))
        .current_dir(repo_root())
        .args(["build", "--release", "-q", "-p", package, "--bin", bin, "--target-dir"])
        .arg(target_dir)
        .env("RUSTFLAGS", "-C target-feature=+crt-static")
        .env("CARGO_PROFILE_RELEASE_DEBUG", "false")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("cargo");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    target_dir.join("release").join(bin)
}

// ANCHOR: initramfs
/// Список файлов initramfs в формате `usr/gen_init_cpio`: каталоги, узел
/// консоли, сам init и модули.
pub struct Initramfs {
    lines: Vec<String>,
}

impl Initramfs {
    pub fn new(init: &Path) -> Initramfs {
        let mut lines: Vec<String> = ["/dev", "/proc", "/sys", "/lib", "/tmp"]
            .iter()
            .map(|d| format!("dir {d} 0755 0 0"))
            .collect();
        // Ядро открывает /dev/console для init до того, как кто-то
        // смонтирует devtmpfs, поэтому узел нужен в самом образе.
        lines.push("nod /dev/console 0600 0 0 c 5 1".into());
        lines.push(format!("file /init {} 0755 0 0", init.display()));
        Initramfs { lines }
    }

    /// Файл `host` окажется в госте как `/lib/<имя>`.
    pub fn lib(mut self, host: &Path) -> Initramfs {
        let name = host.file_name().unwrap().to_string_lossy();
        self.lines.push(format!("file /lib/{name} {} 0644 0 0", host.display()));
        self
    }

    pub fn write(&self, kernel: &Kernel, out: &Path) -> io::Result<()> {
        let list = out.with_extension("list");
        fs::write(&list, self.lines.join("\n") + "\n")?;
        let cpio = Command::new(kernel.gen_init_cpio()).arg(&list).output()?;
        if !cpio.status.success() {
            return Err(io::Error::other(String::from_utf8_lossy(&cpio.stderr).into_owned()));
        }
        fs::write(out, cpio.stdout)
    }
}
// ANCHOR_END: initramfs

// ANCHOR: qemu
/// Запуск QEMU: ядро, initramfs и аргументы init.
pub struct Qemu<'a> {
    pub kernel: &'a Kernel,
    pub initrd: &'a Path,
    /// Аргументы init: в командной строке ядра всё после `--` ядро
    /// передаёт процессу 1 в argv.
    pub args: Vec<String>,
    /// Добавочные параметры ядра.
    pub cmdline: Vec<String>,
    /// Добавочные устройства QEMU: `-device edu`.
    pub devices: Vec<String>,
    /// Прочие аргументы QEMU: `-gdb tcp:…` и `-S` для отладчика.
    pub extra: Vec<String>,
    pub cpus: u32,
    pub mem_mib: u32,
    pub timeout: Duration,
}

impl<'a> Qemu<'a> {
    pub fn new(kernel: &'a Kernel, initrd: &'a Path, args: &[&str]) -> Qemu<'a> {
        // slub_debug=U (запоминать, кто выделил объект) выключает в SLUB
        // ядра 7.0 листы (sheaves) — массивы указателей на свободные
        // объекты у каждого процессора. Выданный объект остаётся в массиве
        // листа старым указателем, kmemleak находит его там и утечку не
        // замечает.
        let cmdline = match kernel.flavor {
            Flavor::Plain => Vec::new(),
            Flavor::Debug => vec!["slub_debug=U".to_string()],
        };
        Qemu {
            kernel,
            initrd,
            args: args.iter().map(|s| s.to_string()).collect(),
            cmdline,
            devices: Vec::new(),
            extra: Vec::new(),
            cpus: 2,
            // KASAN держит теневую память — восьмую часть всей.
            mem_mib: if kernel.flavor == Flavor::Debug { 512 } else { 256 },
            timeout: Duration::from_secs(300),
        }
    }

    /// Вывод последовательного порта — консоль ядра и stdout init.
    pub fn run(&self) -> io::Result<String> {
        // loglevel=1: на консоль только KERN_EMERG; всё остальное init
        // читает из /dev/kmsg сам, чтобы строки не перемешивались.
        let mut append: Vec<String> =
            ["console=ttyS0", "loglevel=1", "panic=-1"].into_iter().map(String::from).collect();
        append.extend(self.cmdline.iter().cloned());
        append.push("--".into());
        append.extend(self.args.iter().cloned());

        // KVM — если /dev/kvm есть и доступен на запись; иначе QEMU
        // эмулирует процессор сам (TCG) — медленнее, но где угодно.
        let kvm = fs::OpenOptions::new().read(true).write(true).open("/dev/kvm").is_ok();
        let accel = if kvm { "kvm" } else { "tcg" };
        let mut cmd = Command::new("qemu-system-x86_64");
        // `max` — все возможности процессора, которые умеет QEMU (или сам
        // процессор под KVM), кроме пятиуровневых таблиц страниц: с LA57
        // ядро раскладывает память иначе, а адреса в отчётах главы — для
        // четырёх уровней (Documentation/arch/x86/x86_64/mm.rst).
        cmd.args(["-machine", &format!("pc,accel={accel}"), "-cpu", "max,la57=off"])
            .args(["-smp", &self.cpus.to_string(), "-m", &format!("{}M", self.mem_mib)])
            .args(["-nographic", "-no-reboot", "-nic", "none"])
            .arg("-kernel")
            .arg(self.kernel.bzimage())
            .arg("-initrd")
            .arg(self.initrd)
            .args(["-append", &append.join(" ")]);
        for d in &self.devices {
            cmd.args(["-device", d]);
        }
        cmd.args(&self.extra);
        let mut child =
            cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        // Вывод читаем в потоке, чтобы QEMU не встал на полном канале.
        let mut stdout = child.stdout.take().unwrap();
        let reader = std::thread::spawn(move || {
            let mut s = Vec::new();
            io::Read::read_to_end(&mut stdout, &mut s).ok();
            s
        });
        let start = Instant::now();
        while child.try_wait()?.is_none() {
            if start.elapsed() > self.timeout {
                child.kill().ok();
                child.wait().ok();
                let out = reader.join().unwrap();
                return Err(io::Error::other(format!(
                    "QEMU не завершился за {:?}:\n{}",
                    self.timeout,
                    String::from_utf8_lossy(&out)
                )));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let out = reader.join().unwrap();
        // Консоль ядра пишет \r\n.
        Ok(String::from_utf8_lossy(&out).replace("\r\n", "\n"))
    }
}
// ANCHOR_END: qemu

/// Строки вывода гостя, которые начинаются с `prefix`, без него.
pub fn lines_with<'s>(serial: &'s str, prefix: &str) -> Vec<&'s str> {
    serial.lines().filter_map(|l| l.strip_prefix(prefix)).collect()
}
