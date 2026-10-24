// SPDX-License-Identifier: GPL-2.0
/* Задача 29.A, фрагмент 2. */
#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/module.h>
#include <linux/slab.h>

struct sensor {
	u32 *samples;
	size_t n;
};

static struct sensor s;

/* ANCHOR: frag */
static void sensor_remove(struct sensor *s)
{
	kfree(s->samples);
	pr_info("последний отсчёт: %u\n", s->samples[s->n - 1]);
	s->samples = NULL;
}
/* ANCHOR_END: frag */

static int __init a2_init(void)
{
	s.n = 8;
	s.samples = kcalloc(s.n, sizeof(*s.samples), GFP_KERNEL);
	if (!s.samples)
		return -ENOMEM;
	s.samples[s.n - 1] = 42;
	sensor_remove(&s);
	return 0;
}

static void __exit a2_exit(void)
{
}

module_init(a2_init);
module_exit(a2_exit);
MODULE_DESCRIPTION("Task 29.A, fragment 2");
MODULE_LICENSE("GPL");
