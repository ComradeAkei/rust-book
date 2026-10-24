// SPDX-License-Identifier: GPL-2.0
/* Задача 29.A, фрагмент 1. */
#include <linux/module.h>
#include <linux/slab.h>
#include <linux/string.h>

struct sensor {
	char *name;
	u32 *samples;
	size_t n;
};

static struct sensor s;

/* ANCHOR: frag */
static int sensor_setup(struct sensor *s, const char *name, size_t n)
{
	s->name = kstrdup(name, GFP_KERNEL);
	if (!s->name)
		return -ENOMEM;
	s->samples = kcalloc(n, sizeof(*s->samples), GFP_KERNEL);
	if (!s->samples) {
		kfree(s->name);
		return -ENOMEM;
	}
	if (n > 64)
		return -EINVAL;
	s->n = n;
	return 0;
}
/* ANCHOR_END: frag */

static int __init a1_init(void)
{
	return sensor_setup(&s, "термометр", 100);
}

static void __exit a1_exit(void)
{
	kfree(s.samples);
	kfree(s.name);
}

module_init(a1_init);
module_exit(a1_exit);
MODULE_DESCRIPTION("Task 29.A, fragment 1");
MODULE_LICENSE("GPL");
