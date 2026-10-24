// SPDX-License-Identifier: GPL-2.0
/* Задача 29.A, фрагмент 3. */
#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/module.h>
#include <linux/mutex.h>

struct sensor {
	struct mutex lock;
	u32 samples[16];
	size_t count;
};

static struct sensor s;

/* ANCHOR: frag */
static int sensor_last(struct sensor *s, u32 *out)
{
	mutex_lock(&s->lock);
	if (s->count == 0)
		return -EAGAIN;
	*out = s->samples[s->count - 1];
	mutex_unlock(&s->lock);
	return 0;
}
/* ANCHOR_END: frag */

static int __init a3_init(void)
{
	u32 v;

	mutex_init(&s.lock);
	pr_info("sensor_last: %d\n", sensor_last(&s, &v));
	return 0;
}

static void __exit a3_exit(void)
{
}

module_init(a3_init);
module_exit(a3_exit);
MODULE_DESCRIPTION("Task 29.A, fragment 3");
MODULE_LICENSE("GPL");
