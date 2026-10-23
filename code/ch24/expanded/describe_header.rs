impl ::ch24::Describe for Header {
    const NAME: &'static ::core::primitive::str = "Header";
    const SIZE: ::core::primitive::usize = ::core::mem::size_of::<Self>();
    const ALIGN: ::core::primitive::usize =
        ::core::mem::align_of::<Self>();
    const FIELDS: &'static [::ch24::Field] =
        &[::ch24::Field {
                        name: "magic",
                        ty: "u16",
                        offset: const { builtin # offset_of(Self, magic) },
                        size: ::core::mem::size_of::<u16>(),
                    },
                    ::ch24::Field {
                        name: "kind",
                        ty: "u8",
                        offset: const { builtin # offset_of(Self, kind) },
                        size: ::core::mem::size_of::<u8>(),
                    },
                    ::ch24::Field {
                        name: "crc",
                        ty: "u32",
                        offset: const { builtin # offset_of(Self, crc) },
                        size: ::core::mem::size_of::<u32>(),
                    }];
    fn dump(&self, out: &mut dyn ::core::fmt::Write)
        -> ::core::fmt::Result {
        ::ch24::dump_header::<Self>(out)?;
        ::ch24::dump_field(out,
                &<Self as ::ch24::Describe>::FIELDS[0usize], &self.magic)?;
        ::ch24::dump_field(out,
                &<Self as ::ch24::Describe>::FIELDS[1usize], &self.kind)?;
        ::ch24::dump_field(out,
                &<Self as ::ch24::Describe>::FIELDS[2usize], &self.crc)?;
        ::core::result::Result::Ok(())
    }
}
const _: () =
    if !(::core::mem::size_of::<Header>() == 8) {
        {
            ::core::panicking::panic_fmt(format_args!("Describe: размер Header не равен 8"));
        }
    };
