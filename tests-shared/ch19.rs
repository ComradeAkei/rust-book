// Общие тесты главы 19 для задач A–D: подключаются через include! в крейты
// заготовок и решений после строки `use <крейт> as sut;`.
//
// Весь код главы 19 — встроенный ассемблер и интринсики x86-64, поэтому все
// тесты помечены #[cfg_attr(miri, ignore)]: Miri не исполняет asm! и часть
// интринсиков. На не-x86-64 крейт не собирается вовсе (см. рабочее
// пространство — ch19 только для хоста).

// --- Задача A: примитивы на asm! ---

#[test]
#[cfg_attr(miri, ignore = "asm!")]
fn a_rotate_left_matches_std() {
    for (x, n) in [(1u64, 1u32), (0xff00, 8), (0x1, 63), (0xdead_beef, 0), (u64::MAX, 40)] {
        assert_eq!(sut::a_asm::rotate_left(x, n), x.rotate_left(n), "rol {x} на {n}");
    }
}

#[test]
#[cfg_attr(miri, ignore = "asm!")]
fn a_leading_zeros_matches_std() {
    for x in [0u64, 1, 2, 255, 0x8000_0000_0000_0000, 0xdead_beef, u64::MAX] {
        assert_eq!(sut::a_asm::leading_zeros(x), x.leading_zeros(), "lz {x}");
    }
}

#[test]
#[cfg_attr(miri, ignore = "asm!")]
fn a_select_is_branchless_ternary() {
    assert_eq!(sut::a_asm::select(true, 10, 20), 10);
    assert_eq!(sut::a_asm::select(false, 10, 20), 20);
}

// --- Задача B: функция в global_asm! ---

#[test]
#[cfg_attr(miri, ignore = "global_asm!")]
fn b_max_matches_std() {
    for (a, b) in [(1u64, 2u64), (9, 3), (5, 5), (0, u64::MAX), (u64::MAX, 0)] {
        assert_eq!(sut::b_global::max(a, b), a.max(b), "max {a} {b}");
    }
}

// --- Задача C: поиск байта на SIMD с диспетчеризацией ---

#[test]
#[cfg_attr(miri, ignore = "интринсики SIMD")]
fn c_index_of_matches_scalar() {
    // Длина с запасом перекрывает и SSE2 (16), и AVX2 (32), и хвост.
    let hay: Vec<u8> = (0..100u32).map(|x| (x % 7) as u8).collect();
    for needle in 0..10u8 {
        let expected = hay.iter().position(|&b| b == needle);
        assert_eq!(sut::c_memchr::index_of(&hay, needle), expected, "needle {needle}");
    }
    // Пустой срез и срез короче вектора.
    assert_eq!(sut::c_memchr::index_of(&[], 1), None);
    assert_eq!(sut::c_memchr::index_of(&[9, 8, 7], 7), Some(2));
    assert_eq!(sut::c_memchr::index_of(&[9, 8, 7], 1), None);
}

#[test]
#[cfg_attr(miri, ignore = "интринсики SIMD")]
fn c_sse2_and_avx2_agree() {
    let hay: Vec<u8> = (0..200u32).map(|x| (x % 13) as u8).collect();
    for needle in 0..14u8 {
        let scalar = hay.iter().position(|&b| b == needle);
        assert_eq!(sut::c_memchr::index_sse2(&hay, needle), scalar, "sse2 {needle}");
        if is_x86_feature_detected!("avx2") {
            // SAFETY: фича проверена прямо перед вызовом.
            let a = unsafe { sut::c_memchr::index_avx2(&hay, needle) };
            assert_eq!(a, scalar, "avx2 {needle}");
        }
    }
}

// --- Задача D: кооперативная сопрограмма ---

#[test]
#[cfg_attr(miri, ignore = "naked-функция и asm!")]
fn d_pingpong_interleaves_main_and_coroutine() {
    assert_eq!(sut::d_coroutine::pingpong(3), vec![1, 2, 1, 2, 1, 2]);
    assert_eq!(sut::d_coroutine::pingpong(1), vec![1, 2]);
    // Повторный вызов начинает журнал заново.
    assert_eq!(sut::d_coroutine::pingpong(2), vec![1, 2, 1, 2]);
}
