// SPDX-License-Identifier: GPL-2.0
/* Задача 29.A, фрагмент 4. */
#include <linux/delay.h>
#include <linux/module.h>
#include <linux/spinlock.h>

struct sensor {
	spinlock_t lock;
	u32 pending;
};

static struct sensor s;

/* ANCHOR: frag */
static void sensor_flush(struct sensor *s)
{
	spin_lock(&s->lock);
	while (s->pending) {
		msleep(1);	/* устройству нужна миллисекунда на отсчёт */
		s->pending--;
	}
	spin_unlock(&s->lock);
}
/* ANCHOR_END: frag */

static int __init a4_init(void)
{
	spin_lock_init(&s.lock);
	s.pending = 2;
	sensor_flush(&s);
	return 0;
}

static void __exit a4_exit(void)
{
}

module_init(a4_init);
module_exit(a4_exit);
MODULE_DESCRIPTION("Task 29.A, fragment 4");
MODULE_LICENSE("GPL");
