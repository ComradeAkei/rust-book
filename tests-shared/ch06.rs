// Общие тесты задач главы 6.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch06/tests/ch06.rs` и `solutions/ch06/tests/ch06.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов.

mod task_a {
    use super::sut::a_predict::ANSWERS;
    use ch06::task_a::QUESTIONS;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn predictions_match_x86_64() {
        assert_eq!(ANSWERS.len(), QUESTIONS.len());
        let mut wrong = Vec::new();
        for (i, (q, &(size, align))) in QUESTIONS.iter().zip(ANSWERS.iter()).enumerate() {
            if (q.size, q.align) != (size, align) {
                wrong.push(format!(
                    "№{:<2} {:<24} ваш ответ ({size}, {align}), на самом деле ({}, {})",
                    i + 1,
                    q.ty,
                    q.size,
                    q.align
                ));
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }
}

mod task_b {
    use super::sut::b_headers::*;

    /// Пример заголовка из статьи Wikipedia «IPv4 header checksum».
    const WIKI_HEADER: [u8; 20] = [
        0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0xb8, 0x61, 0xc0, 0xa8, 0x00,
        0x01, 0xc0, 0xa8, 0x00, 0xc7,
    ];

    /// SYN 192.168.0.1:12345 → 192.168.0.2:80 с опцией MSS 1460, собран scapy.
    const SYN: [u8; 44] = [
        0x45, 0x00, 0x00, 0x2c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x06, 0x9d, 0x32, 0xc0, 0xa8, 0x00,
        0x01, 0xc0, 0xa8, 0x00, 0x02, 0x30, 0x39, 0x00, 0x50, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00,
        0x00, 0x00, 0x60, 0x02, 0xfa, 0xf0, 0xe7, 0x52, 0x00, 0x00, 0x02, 0x04, 0x05, 0xb4,
    ];

    /// PSH+ACK 10.0.0.7:443 → 10.0.0.1:50000 с полезной нагрузкой «hello»
    /// (нечётная длина), DSCP 46, собран scapy.
    const PSH: [u8; 45] = [
        0x45, 0xb8, 0x00, 0x2d, 0x00, 0x07, 0x00, 0x00, 0x05, 0x06, 0xa1, 0x05, 0x0a, 0x00, 0x00,
        0x07, 0x0a, 0x00, 0x00, 0x01, 0x01, 0xbb, 0xc3, 0x50, 0x00, 0x00, 0x03, 0xe8, 0x00, 0x00,
        0x07, 0xd0, 0x50, 0x18, 0x02, 0x00, 0x85, 0x2a, 0x00, 0x00, 0x68, 0x65, 0x6c, 0x6c, 0x6f,
    ];

    fn wiki_packet() -> Vec<u8> {
        let mut p = WIKI_HEADER.to_vec();
        p.resize(0x73, 0);
        p
    }

    #[test]
    fn checksum_of_valid_header_is_zero() {
        assert_eq!(internet_checksum(&WIKI_HEADER), 0);
        let mut zeroed = WIKI_HEADER;
        zeroed[10] = 0;
        zeroed[11] = 0;
        assert_eq!(internet_checksum(&zeroed), 0xb861);
    }

    #[test]
    fn checksum_odd_length() {
        // RFC 1071: нечётный хвост дополняется нулевым байтом справа.
        assert_eq!(internet_checksum(&[0x01]), !0x0100);
        assert_eq!(internet_checksum(&[0x01, 0x02, 0x03]), !(0x0102u16 + 0x0300));
    }

    #[test]
    fn parse_wiki_header() {
        let p = wiki_packet();
        let h = parse_ipv4(&p).unwrap();
        assert_eq!(h.total_len, 0x73);
        assert_eq!(h.ttl, 64);
        assert_eq!(h.protocol, 17);
        assert!(h.dont_fragment);
        assert!(!h.more_fragments);
        assert_eq!(h.frag_offset, 0);
        assert_eq!(h.src, [192, 168, 0, 1]);
        assert_eq!(h.dst, [192, 168, 0, 199]);
        assert_eq!(h.checksum, 0xb861);
        assert!(h.options.is_empty());
        assert_eq!(h.payload.len(), 0x73 - 20);
    }

    #[test]
    fn trailing_bytes_are_not_payload() {
        let mut p = wiki_packet();
        p.extend_from_slice(&[0xAA; 6]); // дополнение кадра Ethernet
        assert_eq!(parse_ipv4(&p).unwrap().payload.len(), 0x73 - 20);
    }

    #[test]
    fn ipv4_errors() {
        assert_eq!(parse_ipv4(&WIKI_HEADER[..19]), Err(HeaderError::Truncated { need: 20, have: 19 }));
        assert_eq!(parse_ipv4(&WIKI_HEADER), Err(HeaderError::Truncated { need: 0x73, have: 20 }));

        let mut p = wiki_packet();
        p[0] = 0x65;
        assert_eq!(parse_ipv4(&p), Err(HeaderError::Version(6)));

        let mut p = wiki_packet();
        p[0] = 0x44;
        assert_eq!(parse_ipv4(&p), Err(HeaderError::HeaderLen(4)));

        let mut p = wiki_packet();
        p[2] = 0;
        p[3] = 19;
        assert_eq!(parse_ipv4(&p), Err(HeaderError::TotalLen(19)));

        let mut p = wiki_packet();
        p[8] = 63; // TTL уменьшили, а сумму не пересчитали
        assert_eq!(
            parse_ipv4(&p),
            Err(HeaderError::Checksum { stored: 0xb861, computed: 0xb961 })
        );
    }

    #[test]
    fn header_with_options() {
        // IHL = 6: одна 4-байтовая опция (NOP, NOP, NOP, EOL).
        let mut p = vec![0x46, 0, 0, 28, 0, 1, 0, 0, 64, 17, 0, 0, 10, 0, 0, 1, 10, 0, 0, 2];
        p.extend_from_slice(&[1, 1, 1, 0]);
        p.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let sum = internet_checksum(&p[..24]);
        p[10..12].copy_from_slice(&sum.to_be_bytes());
        let h = parse_ipv4(&p).unwrap();
        assert_eq!(h.options, &[1, 1, 1, 0]);
        assert_eq!(h.payload, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn parse_syn() {
        let ip = parse_ipv4(&SYN).unwrap();
        assert_eq!(ip.protocol, 6);
        assert_eq!(ip.ident, 0x1c46);
        let tcp = parse_tcp(ip.payload).unwrap();
        assert_eq!(tcp.src_port, 12345);
        assert_eq!(tcp.dst_port, 80);
        assert_eq!(tcp.seq, 0x0102_0304);
        assert_eq!(tcp.ack, 0);
        assert!(tcp.flags.contains(TcpFlags::SYN));
        assert!(!tcp.flags.contains(TcpFlags::ACK));
        assert_eq!(tcp.window, 64240);
        assert_eq!(tcp.options, &[0x02, 0x04, 0x05, 0xb4]);
        assert!(tcp.payload.is_empty());
        assert_eq!(tcp_checksum(ip.src, ip.dst, ip.payload), Ok(tcp.checksum));
        assert_eq!(tcp.checksum, 0xe752);
    }

    #[test]
    fn parse_psh_odd_payload() {
        let ip = parse_ipv4(&PSH).unwrap();
        assert_eq!(ip.dscp, 46);
        assert_eq!(ip.ecn, 0);
        assert!(!ip.dont_fragment);
        let tcp = parse_tcp(ip.payload).unwrap();
        assert!(tcp.flags.contains(TcpFlags::PSH | TcpFlags::ACK));
        assert_eq!(tcp.payload, b"hello");
        assert_eq!(tcp_checksum(ip.src, ip.dst, ip.payload), Ok(0x852a));
    }

    #[test]
    fn tcp_errors() {
        assert_eq!(parse_tcp(&SYN[20..39]), Err(HeaderError::Truncated { need: 20, have: 19 }));
        let mut seg = SYN[20..].to_vec();
        seg[12] = 0x40; // data offset 4
        assert_eq!(parse_tcp(&seg), Err(HeaderError::DataOffset(4)));
        let mut seg = SYN[20..].to_vec();
        seg[12] = 0x70; // data offset 7 = 28 байт, а сегмент 24
        assert_eq!(parse_tcp(&seg), Err(HeaderError::Truncated { need: 28, have: 24 }));
    }

    /// Буфер, у которого гарантированно можно получить и чётный, и нечётный адрес.
    #[repr(C, align(4))]
    struct Aligned([u8; 48]);

    #[test]
    fn raw_view_checks_alignment() {
        let mut buf = Aligned([0; 48]);
        buf.0[..20].copy_from_slice(&WIKI_HEADER);
        let h = view_ipv4(&buf.0[..20]).unwrap();
        assert_eq!(h.version(), 4);
        assert_eq!(h.header_len(), 20);
        assert_eq!(h.total_len(), 0x73);
        assert_eq!(h.checksum(), 0xb861);
        assert!(h.dont_fragment());

        buf.0[1..21].copy_from_slice(&WIKI_HEADER);
        let odd = &buf.0[1..21];
        assert_eq!(
            view_ipv4(odd).unwrap_err(),
            ViewError::Misaligned { addr: odd.as_ptr() as usize, align: 2 }
        );
        assert_eq!(view_ipv4(&buf.0[..10]).unwrap_err(), ViewError::Truncated { need: 20, have: 10 });
    }
}

mod task_c {
    use super::sut::c_register::*;
    use ch06_regs::Pl011;

    const ALL_WLEN: [WordLen; 4] = [WordLen::Five, WordLen::Six, WordLen::Seven, WordLen::Eight];
    const ALL_PARITY: [Parity; 5] = [Parity::None, Parity::Odd, Parity::Even, Parity::Mark, Parity::Space];

    #[test]
    fn wrapper_is_free() {
        assert_eq!(core::mem::size_of::<LcrH>(), 4);
        assert_eq!(core::mem::align_of::<LcrH>(), 4);
    }

    #[test]
    fn documented_values() {
        let v = LcrH::RESET.with_word_len(WordLen::Eight).with_fifo(true);
        assert_eq!(v.bits(), 0x70);
        let v = v.with_parity(Parity::Even).with_stop_bits(StopBits::Two);
        assert_eq!(v.bits(), 0x70 | 0b0000_0110 | 0b0000_1000);
        assert_eq!(LcrH::RESET.with_break(true).bits(), 1);
    }

    #[test]
    fn decode_reset() {
        let r = LcrH::RESET;
        assert_eq!(r.word_len(), WordLen::Five);
        assert_eq!(r.parity(), Parity::None);
        assert_eq!(r.stop_bits(), StopBits::One);
        assert!(!r.fifo_enabled());
        assert!(!r.sends_break());
    }

    #[test]
    fn eps_without_pen_means_no_parity() {
        assert_eq!(LcrH::from_bits(0b1000_0100).parity(), Parity::None);
        assert_eq!(LcrH::from_bits(0b1000_0100).with_parity(Parity::None).bits(), 0);
    }

    #[test]
    fn setters_touch_only_their_bits() {
        // LCR_H использует 8 младших битов, поэтому перебор всех 256
        // исходных значений покрывает каждое сочетание полей.
        for raw in 0u32..=0xFF {
            let r = LcrH::from_bits(raw | 0xFFFF_FF00);
            for w in ALL_WLEN {
                let n = r.with_word_len(w);
                assert_eq!(n.word_len(), w);
                assert_eq!(n.bits() & !0x60, r.bits() & !0x60, "WLEN задел чужие биты: {raw:#04x}");
            }
            for p in ALL_PARITY {
                let n = r.with_parity(p);
                assert_eq!(n.parity(), p);
                assert_eq!(n.bits() & !0x86, r.bits() & !0x86, "чётность задела чужие биты: {raw:#04x}");
            }
            for on in [false, true] {
                assert_eq!(r.with_fifo(on).fifo_enabled(), on);
                assert_eq!(r.with_fifo(on).bits() & !0x10, r.bits() & !0x10);
                assert_eq!(r.with_break(on).sends_break(), on);
                assert_eq!(r.with_break(on).bits() & !0x01, r.bits() & !0x01);
            }
            for s in [StopBits::One, StopBits::Two] {
                assert_eq!(r.with_stop_bits(s).stop_bits(), s);
                assert_eq!(r.with_stop_bits(s).bits() & !0x08, r.bits() & !0x08);
            }
        }
    }

    #[test]
    fn typed_and_raw_configure_agree() {
        let a = Pl011::zeroed();
        let b = Pl011::zeroed();
        ch06c_configure_raw(&a);
        ch06c_configure_typed(&b);
        assert_eq!(a.lcr_h.read(), b.lcr_h.read());
        for w in ALL_WLEN {
            a.lcr_h.write(0xFF);
            b.lcr_h.write(0xFF);
            ch06c_set_word_len_raw(&a, w);
            ch06c_set_word_len_typed(&b, w);
            assert_eq!(a.lcr_h.read(), b.lcr_h.read());
        }
    }
}

mod task_d {
    use super::sut::d_command::*;
    use ch06::task_d::{NaiveCmd, WINDOW_BASE, WINDOW_LEN};
    use core::mem::size_of;

    #[test]
    fn sizes() {
        assert_eq!(size_of::<Cmd>(), 8, "Cmd должен занимать 8 байт");
        assert_eq!(size_of::<Option<Cmd>>(), 8, "Option<Cmd> должен использовать нишу");
        assert_eq!(size_of::<Result<Cmd, ConvertError>>(), 8, "ошибка не должна раздувать Result");
    }

    fn valid_commands() -> Vec<NaiveCmd> {
        let mut v = vec![
            NaiveCmd::Ping,
            NaiveCmd::Reset { delay_ms: 0 },
            NaiveCmd::Reset { delay_ms: u32::MAX },
            NaiveCmd::Read { addr: WINDOW_BASE },
            NaiveCmd::Read { addr: WINDOW_BASE + WINDOW_LEN - 4 },
            NaiveCmd::Write { addr: 0x4000_C000, value: 0xDEAD_BEEF },
            NaiveCmd::Write { addr: WINDOW_BASE + WINDOW_LEN - 4, value: 0 },
        ];
        for id in 0..=255u8 {
            v.push(NaiveCmd::SetLed { id, on: id % 3 == 0 });
        }
        for channel in [0u8, 1, 7, 255] {
            for duty in [0u16, 1, 0x8000, u16::MAX] {
                v.push(NaiveCmd::SetPwm { channel, duty });
            }
        }
        v
    }

    #[test]
    fn round_trip() {
        for n in valid_commands() {
            let c = Cmd::try_from(n).unwrap_or_else(|e| panic!("{n:?} отвергнута: {e:?}"));
            assert_eq!(NaiveCmd::from(c), n);
        }
    }

    #[test]
    fn invalid_addresses_report_kind_and_addr() {
        let cases = [
            (WINDOW_BASE + 2, ConvertErrorKind::Misaligned),
            (WINDOW_BASE + WINDOW_LEN, ConvertErrorKind::OutOfWindow),
            (WINDOW_BASE - 4, ConvertErrorKind::OutOfWindow),
            (0, ConvertErrorKind::OutOfWindow),
            (u32::MAX - 3, ConvertErrorKind::OutOfWindow),
        ];
        for (addr, kind) in cases {
            for n in [NaiveCmd::Read { addr }, NaiveCmd::Write { addr, value: 1 }] {
                let e = Cmd::try_from(n).expect_err("адрес вне окна должен отвергаться");
                assert_eq!((e.kind(), e.addr()), (kind, addr), "{n:?}");
            }
        }
    }
}
