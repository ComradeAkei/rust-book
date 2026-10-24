// SPDX-License-Identifier: GPL-2.0
/* Задача 29.A, фрагмент 6. */
#include <linux/module.h>
#include <linux/mutex.h>

struct sensor {
	struct mutex lock;
	u32 samples;
};

static struct sensor inner, outer;

/* ANCHOR: frag */
static void sensor_move(struct sensor *from, struct sensor *to)
{
	mutex_lock(&from->lock);
	mutex_lock(&to->lock);
	to->samples += from->samples;
	from->samples = 0;
	mutex_unlock(&to->lock);
	mutex_unlock(&from->lock);
}
/* ANCHOR_END: frag */

static int __init a6_init(void)
{
	mutex_init(&inner.lock);
	mutex_init(&outer.lock);
	inner.samples = 3;
	sensor_move(&inner, &outer);	/* сбор отсчётов */
	sensor_move(&outer, &inner);	/* возврат при сбросе */
	return 0;
}

static void __exit a6_exit(void)
{
}

module_init(a6_init);
module_exit(a6_exit);
MODULE_DESCRIPTION("Task 29.A, fragment 6");
MODULE_LICENSE("GPL");
