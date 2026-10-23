// Общие тесты главы 17 для задач B, C и D: подключаются через include! в
// крейты заготовок и решений после строки `use <крейт> as sut;`.
//
// Задача A (каталог transmute) живёт в solutions/ch17/tests/ub.rs:
// корректные вызовы — обычные тесты, UB — #[ignore] под Miri.

use std::cell::Cell;
use std::rc::Rc;

// --- Задача B: разбор заголовка ELF64 ---

#[repr(C, align(8))]
struct Aligned([u8; 64]);

fn sample_elf() -> Aligned {
    let mut b = Aligned([0u8; 64]);
    b.0[..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    b.0[4] = 2; // ELFCLASS64
    b.0[5] = 1; // ELFDATA2LSB
    b.0[6] = 1; // EV_CURRENT
    b.0[16..18].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
    b.0[18..20].copy_from_slice(&0x3eu16.to_le_bytes()); // EM_X86_64
    b.0[20..24].copy_from_slice(&1u32.to_le_bytes());
    b.0[24..32].copy_from_slice(&0x40_1000u64.to_le_bytes());
    b
}

#[test]
fn b_both_parsers_agree() {
    let b = sample_elf();
    let buf = &b.0[..];
    let zc = sut::b_elf::parse_zero_copy(buf).expect("zero-copy");
    let sf = sut::b_elf::parse_safe(buf).expect("safe");
    assert_eq!(*zc, sf);
    assert_eq!(zc.e_type, 2);
    assert_eq!(zc.machine, 0x3e);
    assert_eq!(zc.entry, 0x40_1000);
}

#[test]
fn b_rejects_non_elf_and_short() {
    let mut bad = sample_elf();
    bad.0[0] = 0; // испортить магию
    assert!(sut::b_elf::parse_zero_copy(&bad.0[..]).is_none());
    assert!(sut::b_elf::parse_safe(&bad.0[..]).is_none());

    let b = sample_elf();
    assert!(sut::b_elf::parse_safe(&b.0[..10]).is_none()); // короткий буфер
}

#[test]
fn b_zero_copy_rejects_unaligned() {
    let b = sample_elf();
    // срез со смещением 1 от выровненного на 8 буфера заведомо невыровнен
    assert!(sut::b_elf::parse_zero_copy(&b.0[1..33]).is_none());
}

// --- Задача C: тегированный указатель ---

#[test]
fn c_tag_and_pointer_roundtrip() {
    let node = Box::new(sut::c_tagged::Node { value: 0xC0FFEE });
    let p: *const sut::c_tagged::Node = &*node;
    for tag in 0..8 {
        let t = sut::c_tagged::Tagged::new(p, tag);
        assert_eq!(t.tag(), tag);
        // SAFETY: ptr() несёт provenance исходного p, node жив.
        assert_eq!(unsafe { &*t.ptr() }.value, 0xC0FFEE);
    }
}

// --- Задача D: свой DST и сравнение с Box<[T]> ---

struct Dropper(Rc<Cell<u32>>);
impl Drop for Dropper {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn d_record_holds_and_drops_each_once() {
    let c = Rc::new(Cell::new(0u32));
    let items = vec![Dropper(c.clone()), Dropper(c.clone()), Dropper(c.clone())];
    let rec = sut::d_record::Record::new(7, items);
    assert_eq!(rec.tag, 7);
    assert_eq!(rec.data.len(), 3);
    drop(rec);
    assert_eq!(c.get(), 3, "каждый элемент хвоста должен быть уронен ровно один раз");
}

#[test]
fn d_twobox_same_data() {
    let c = Rc::new(Cell::new(0u32));
    let tb = sut::d_record::TwoBox::new(9, vec![Dropper(c.clone()), Dropper(c.clone())]);
    assert_eq!(tb.tag, 9);
    assert_eq!(tb.data.len(), 2);
    drop(tb);
    assert_eq!(c.get(), 2);
}
