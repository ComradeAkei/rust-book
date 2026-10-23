#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
// Раздел 9.6: что генерирует derive. Снимок рядом, в derive.out.rs:
// RUSTC_BOOTSTRAP=1 rustc --edition 2024 --crate-type lib -Zunpretty=expanded derive.rs
pub struct Sample {
    pub sensor: u16,
    pub value: i32,
}
#[automatically_derived]
impl ::core::fmt::Debug for Sample {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(f, "Sample",
            "sensor", &self.sensor, "value", &&self.value)
    }
}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for Sample { }
#[automatically_derived]
impl ::core::clone::Clone for Sample {
    #[inline]
    fn clone(&self) -> Sample {
        let _: ::core::clone::AssertParamIsClone<u16>;
        let _: ::core::clone::AssertParamIsClone<i32>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for Sample { }
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Sample { }
#[automatically_derived]
impl ::core::cmp::PartialEq for Sample {
    #[inline]
    fn eq(&self, other: &Sample) -> bool {
        self.sensor == other.sensor && self.value == other.value
    }
}
#[automatically_derived]
impl ::core::default::Default for Sample {
    #[inline]
    fn default() -> Sample {
        Sample {
            sensor: ::core::default::Default::default(),
            value: ::core::default::Default::default(),
        }
    }
}
