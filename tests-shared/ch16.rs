// Общие тесты главы 16 для задач B и C: подключаются через include! в
// крейты заготовок и решений, после строки `use <крейт> as sut;`.
//
// Задача B проверяет, что `split_first_mut` отдаёт голову и хвост как две
// живые непересекающиеся `&mut`. Задача C — что честная сигнатура `view`
// отдаёт содержимое буфера. Задачи A (каталог UB) и D (расхождение моделей
// алиасинга) не проверяются обычным тестом: они содержат неопределённое
// поведение или зависят от модели и живут в solutions/ch16/tests/ub.rs,
// где их гоняют под Miri поодиночке.

// --- Задача B: split_first_mut ---

#[test]
fn b_empty_is_none() {
    let mut v: [i32; 0] = [];
    assert!(sut::b_split::split_first_mut(&mut v).is_none());
}

#[test]
fn b_splits_head_and_tail() {
    let mut v = [1, 2, 3, 4];
    let (head, tail) = sut::b_split::split_first_mut(&mut v).unwrap();
    assert_eq!(*head, 1);
    assert_eq!(tail, &mut [2, 3, 4]);
    // Обе ссылки живы одновременно и указывают на разную память.
    *head = 10;
    tail[0] = 20;
    assert_eq!(v, [10, 20, 3, 4]);
}

#[test]
fn b_single_element_has_empty_tail() {
    let mut v = [42];
    let (head, tail) = sut::b_split::split_first_mut(&mut v).unwrap();
    assert_eq!(*head, 42);
    assert!(tail.is_empty());
}

// --- Задача C: честная сигнатура StrBuf::view ---

#[test]
fn c_view_matches_contents() {
    let buf = sut::c_sound::StrBuf::new("i2c");
    assert_eq!(buf.view(), "i2c");
    // Взгляд живёт, пока жив buf, — и ни секундой дольше: это держит
    // заёмщик, здесь мы просто пользуемся им в области видимости buf.
    let v = buf.view();
    assert_eq!(v.len(), 3);
}
