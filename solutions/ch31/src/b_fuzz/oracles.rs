// Оракулы задачи 31.B: паника — найденная ошибка. Файл подключается через
// `include!` в модуль, где рядом лежат `ip` и `lexer`, — в исправленный
// (`b_fuzz`) и в заготовку с внедрёнными ошибками (`b_fuzz::injected`).

// ANCHOR: packet
/// Любые байты — как пакет IPv4 и как сегмент TCP. Разбор не паникует, а
/// принятый заголовок согласован с байтами, из которых он разобран.
pub fn check_packet(data: &[u8]) {
    if let Ok(h) = ip::parse_ipv4(data) {
        let total = usize::from(h.total_len);
        let header = 20 + h.options.len();
        assert!(total <= data.len(), "Total Length {total} за концом буфера {}", data.len());
        assert_eq!(header + h.payload.len(), total, "заголовок и нагрузка — не весь пакет");
        assert_eq!(h.options, &data[20..header]);
        assert_eq!(h.payload, &data[header..total]);
        assert_eq!(ip::internet_checksum(&data[..header]), 0, "принят заголовок с неверной суммой");
        if h.protocol == 6 {
            check_segment(h.payload);
        }
    }
    check_segment(data);
}

fn check_segment(seg: &[u8]) {
    if let Ok(t) = ip::parse_tcp(seg) {
        let header = 20 + t.options.len();
        assert_eq!(usize::from(seg[12] >> 4) * 4, header, "Data Offset не равен длине заголовка");
        assert_eq!(header + t.payload.len(), seg.len(), "заголовок и нагрузка — не весь сегмент");
        assert_eq!(t.payload, &seg[header..]);
    }
}
// ANCHOR_END: packet

// ANCHOR: config
/// Любой текст. Токенизатор не паникует; за ключом всегда идёт значение;
/// каждая лексема — срез исходного текста, а за значением в его строке —
/// только закрывающая кавычка, пробелы и комментарий; пробелов по краям у
/// значения без кавычек нет (в кавычках они — часть значения); ошибка
/// указывает на существующую строку.
pub fn check_config(text: &str) {
    let lines = text.split('\n').count();
    let mut tokens = lexer::Lexer::new(text);
    while let Some(item) = tokens.next() {
        match item {
            Ok(lexer::Token::Section(name)) => {
                offset(text, name);
            }
            Ok(lexer::Token::Key(key)) => {
                offset(text, key);
                let Some(Ok(lexer::Token::Value(v))) = tokens.next() else {
                    panic!("за ключом {key:?} нет значения");
                };
                let start = offset(text, v);
                let quoted = text[..start].ends_with('"');
                let line = text[start + v.len()..].split('\n').next().unwrap_or("");
                let rest = if quoted {
                    let Some(rest) = line.strip_prefix('"') else {
                        panic!("значение {v:?} в кавычках оборвано: дальше {line:?}");
                    };
                    rest
                } else {
                    line
                };
                let rest = rest.trim_start();
                assert!(
                    rest.is_empty() || rest.starts_with(['#', ';']),
                    "значение {v:?} оборвано: дальше в строке {rest:?}"
                );
                if !quoted {
                    assert_eq!(v, v.trim(), "пробелы по краям значения без кавычек");
                }
            }
            Ok(lexer::Token::Value(v)) => panic!("значение {v:?} без ключа"),
            Err(e) => assert!(
                (1..=lines).contains(&(e.line as usize)) && e.col >= 1,
                "ошибка вне текста: {e:?}"
            ),
        }
    }
}

/// Смещение среза `s` в тексте; паника, если срез не из текста.
fn offset(text: &str, s: &str) -> usize {
    let (base, p) = (text.as_ptr() as usize, s.as_ptr() as usize);
    assert!(base <= p && p + s.len() <= base + text.len(), "лексема {s:?} не из текста");
    p - base
}
// ANCHOR_END: config
