//! Запускает `exit-skips-drop` и проверяет, что буфер потерян.

#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn exit_skips_destructors() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_exit-skips-drop"))
        .output()
        .expect("программа из этого же пакета должна запускаться");
    assert!(out.status.success());
    assert_eq!(out.stdout, b"");
}
