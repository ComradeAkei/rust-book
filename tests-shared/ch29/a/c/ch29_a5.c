// SPDX-License-Identifier: GPL-2.0
/* Задача 29.A, фрагмент 5. */
#include <linux/module.h>
#include <linux/slab.h>

struct sensor {
	u32 *samples;
	size_t n;
};

static struct sensor s;

/* ANCHOR: frag */
static int sensor_store(struct sensor *s, size_t i, u32 v)
{
	if (i > s->n)
		return -EINVAL;
	s->samples[i] = v;
	return 0;
}
/* ANCHOR_END: frag */

static int __init a5_init(void)
{
	size_t i;

	s.n = 8;
	s.samples = kcalloc(s.n, sizeof(*s.samples), GFP_KERNEL);
	if (!s.samples)
		return -ENOMEM;
	for (i = 0; i <= s.n; i++)
		sensor_store(&s, i, i * 10);
	return 0;
}

static void __exit a5_exit(void)
{
	kfree(s.samples);
}

module_init(a5_init);
module_exit(a5_exit);
MODULE_DESCRIPTION("Task 29.A, fragment 5");
MODULE_LICENSE("GPL");
