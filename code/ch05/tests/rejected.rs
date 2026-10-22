//! Раздел 5.6: программа из `code/ch05/rejected` отвергается, и компилятор
//! объясняет, что граница `for<'a>` на GAT подразумевает `'static`.

use std::path::Path;
use std::process::Command;

#[test]
#[cfg_attr(miri, ignore = "запускает rustc")]
fn gat_hrtb_implies_static() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new("rustc")
        .current_dir(dir)
        .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "-o"])
        .arg(std::env::temp_dir().join(format!("ch05-rejected-{}.rmeta", std::process::id())))
        .arg("rejected/gat_static.rs")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "программа скомпилировалась");
    assert!(text.contains("error[E0521]: borrowed data escapes outside of function"), "{text}");
    assert!(text.contains("argument requires that `'1` must outlive `'static`"), "{text}");
    assert!(
        text.contains(
            "due to a current limitation of the type system, this implies a `'static` lifetime"
        ),
        "{text}"
    );
}
