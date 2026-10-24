//! Сквозной проект главы 27 (раздел 27.9) со стороны хоста: прошивка
//! станции отправляет кадры в UART0, QEMU пишет их в файл, а тест
//! разбирает файл тем же `telemetry-frame`, которым кодирует прошивка.
//! Тесту нужны qemu-system-arm и цель thumbv7m-none-eabi, поэтому он
//! помечен `#[ignore]` и запускается в CI в задании Cortex-M:
//!
//! ```text
//! cargo test -p ch27 --test station -- --ignored
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use telemetry_frame::stream::{Frames, Stats, stats};
use telemetry_frame::{Kind, Measurement};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Собрать прошивку закреплённым cargo в отдельный каталог. RUSTFLAGS
/// убирается: она заменила бы флаги из `firmware/.cargo/config.toml`.
fn build_station(target_dir: &Path) -> PathBuf {
    let out = Command::new("cargo")
        .current_dir(root().join("firmware"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["build", "--release", "-p", "ch27-station", "--target-dir"])
        .arg(target_dir)
        .output()
        .expect("не удалось запустить cargo");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    target_dir.join("thumbv7m-none-eabi/release/ch27-station")
}

/// QEMU с UART0 в файл `uart`; возвращает вывод semihosting.
fn run_qemu(elf: &Path, uart: &Path) -> String {
    let mut child = Command::new("qemu-system-arm")
        .args(["-cpu", "cortex-m3", "-machine", "lm3s6965evb", "-nographic", "-monitor", "none"])
        .args(["-semihosting-config", "enable=on,target=native"])
        .arg("-serial")
        .arg(format!("file:{}", uart.display()))
        .arg("-kernel")
        .arg(elf)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("нет qemu-system-arm");
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(60) {
            child.kill().ok();
            panic!("QEMU не завершился за минуту");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "QEMU: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ANCHOR: host
#[test]
#[ignore = "собирает прошивку и запускает QEMU"]
fn qemu_station_frames() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch27-station");
    std::fs::create_dir_all(&dir).unwrap();
    let uart = dir.join("uart0.bin");
    let report = run_qemu(&build_station(&dir.join("target")), &uart);
    assert!(report.contains("станция: 20 кадров измерений, 360 байт в UART0"), "{report}");
    assert!(report.contains("потеряно в очереди 0"), "{report}");

    let bytes = std::fs::read(&uart).unwrap();
    // Ни мусора между кадрами, ни ошибок CRC: 20 кадров по 18 байт.
    assert_eq!(stats(&bytes), Stats { frames: [20, 0, 0], errors: 0, junk: 0 });
    for (i, item) in Frames::until_eof(&bytes).enumerate() {
        let (offset, frame) = item.unwrap();
        assert_eq!(offset, i * 18);
        assert_eq!((frame.header.kind, usize::from(frame.header.seq)), (Kind::Measurement, i));
        let m = Measurement::from_bytes(frame.payload).unwrap();
        assert_eq!((m.sensor.get(), m.timestamp_ms), (1, 10 * i as u32), "кадр {i}");
        // QEMU настоящего входа не моделирует: отсчёты 512..519 из 1023,
        // то есть 1500..1520 мВ при опорном 3 В.
        assert!((1500..=1520).contains(&m.value), "кадр {i}: {} мВ", m.value);
    }
}
// ANCHOR_END: host
