//! Задача A: правило сироты и когерентность для шести пар реализаций из
//! `code/ch09/fragments`.

/// Что скажет компилятор о паре реализаций.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Обе реализации принимаются.
    Compiles,
    /// Реализация ⟨n⟩ нарушает правило сироты (E0117 или E0210).
    Orphan(u8),
    /// Реализации пересекаются — друг с другом или с реализацией из
    /// другого крейта (E0119).
    Overlap,
}

// ANCHOR: answers
pub fn verdicts() -> [Verdict; 6] {
    use Verdict::*;
    [
        // a1: Vec<Reading> — чужой тип, хоть и с локальным параметром.
        // From<Reading> для Vec<u8> законна: локальный тип в параметре
        // трейта, и до него нет непокрытых параметров.
        Orphan(1),
        // a2: Display у Reading нет, и добавить его может только этот
        // крейт. Компилятор знает это и пересечения не видит.
        Compiles,
        // a3: Box<u8> не Copy сейчас, но реализация Copy for Box<u8> может
        // появиться в alloc. Для чужого типа отрицательного знания нет.
        Overlap,
        // a4: Box и & — фундаментальные типы: Box<Reading> и &Reading
        // считаются локальными. Display для Reading нет, поэтому общая
        // реализация Display for &T из core не пересекается с ⟨2⟩.
        Compiles,
        // a5: ⟨2⟩ даёт Reading: Into<i32> через общую реализацию Into, и
        // ⟨1⟩ при T = Reading совпадает с From<T> for T из core.
        Overlap,
        // a6: в Add<Reading> for T непокрытый T стоит раньше первого
        // локального типа — E0210.
        Orphan(2),
    ]
}
// ANCHOR_END: answers
