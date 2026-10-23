#![feature /* 839#0 */(prelude_import /* 1465#0 */)]
extern crate std /* 1876#1 */;
#[prelude_import /* 1465#1 */]
use std /* 1876#1 */::prelude /* 1464#1 */::rust_2024 /* 1598#1 */::*;
// Раздел 23.3: swap! после раскрытия — с номерами контекстов гигиены.
// Снимок рядом, в swap.out.rs:
// RUSTC_BOOTSTRAP=1 rustc --edition 2024 --crate-type lib -Zunpretty=expanded,hygiene swap.rs
macro_rules! swap
    /*
    2352#0
    */ {
    ($a /* 2247#0 */:ident /* 970#0 */, $b /* 2248#0 */:ident /* 970#0 */) =>
    {
        let /* 17#0 */ tmp /* 2353#0 */ = $a /* 2247#0 */; $a /* 2247#0 */ =
        $b /* 2248#0 */; $b /* 2248#0 */ = tmp /* 2353#0 */;
    };
}

pub fn call /* 470#0
    */(mut x /* 2270#0 */: i32 /* 966#0 */,
    mut tmp /* 2353#0 */: i32 /* 966#0 */)
    -> (i32 /* 966#0 */, i32 /* 966#0 */) {
    let tmp /* 2353#4 */ = x /* 2270#0 */;
    x /* 2270#0 */ = tmp /* 2353#0 */;
    tmp /* 2353#0 */ = tmp /* 2353#4 */;
    ;
    (x /* 2270#0 */, tmp /* 2353#0 */)
}

/*
Expansions:
crate0::{{expn0}}: parent: crate0::{{expn0}}, call_site_ctxt: #0, def_site_ctxt: #0, kind: Root
crate0::{{expn1}}: parent: crate0::{{expn0}}, call_site_ctxt: #0, def_site_ctxt: #0, kind: AstPass(StdImports)
crate0::{{expn2}}: parent: crate0::{{expn0}}, call_site_ctxt: #0, def_site_ctxt: #0, kind: Macro(Bang, "swap")
crate1::{{expn1}}: parent: crate0::{{expn0}}, call_site_ctxt: #0, def_site_ctxt: #0, kind: AstPass(StdImports)

SyntaxContexts:
#0: parent: #0, outer_mark: (crate0::{{expn0}}, Opaque)
#1: parent: #0, outer_mark: (crate0::{{expn1}}, Opaque)
#2: parent: #0, outer_mark: (crate0::{{expn1}}, Transparent)
#3: parent: #0, outer_mark: (crate1::{{expn1}}, Opaque)
#4: parent: #0, outer_mark: (crate0::{{expn2}}, SemiOpaque)
*/
