// SPDX-License-Identifier: GPL-2.0

//! 29.3: минимальный модуль с параметром.

// ANCHOR: all
use kernel::prelude::*;

module! {
    type: Hello,
    name: "ch29_hello",
    authors: ["Rust textbook"],
    description: "Minimal module with a parameter",
    license: "GPL",
    params: {
        times: u32 {
            default: 1,
            description: "How many times to say hello",
        },
    },
}

struct Hello {
    times: u32,
}

impl kernel::Module for Hello {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        let times = *module_parameters::times.value();
        if times > 10 {
            pr_err!("times = {times}: не больше 10\n");
            return Err(EINVAL);
        }
        for i in 1..=times {
            pr_info!("привет #{i}\n");
        }
        Ok(Hello { times })
    }
}

impl Drop for Hello {
    fn drop(&mut self) {
        pr_info!("пока, было приветствий: {}\n", self.times);
    }
}
// ANCHOR_END: all
