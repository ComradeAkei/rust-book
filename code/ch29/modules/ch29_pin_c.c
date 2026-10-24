// SPDX-License-Identifier: GPL-2.0
/*
 * 29.5: что бывает, если переместить инициализированный мьютекс в C.
 * memcpy копирует и указатели, которые смотрят внутрь оригинала.
 */
#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/string.h>

/* ANCHOR: copy */
struct journal {
	const char *name;
	struct mutex lock;
	unsigned int count;
};

static struct journal orig = { .name = "кадры" };
static struct journal moved;

static int __init pin_c_init(void)
{
	mutex_init(&orig.lock);
	pr_info("оригинал: wait_list %px, next %px\n",
		&orig.lock.wait_list, orig.lock.wait_list.next);

	memcpy(&moved, &orig, sizeof(orig));	/* «переместили» журнал */
	pr_info("копия:    wait_list %px, next %px\n",
		&moved.lock.wait_list, moved.lock.wait_list.next);

	mutex_lock(&moved.lock);
	moved.count++;
	mutex_unlock(&moved.lock);
	pr_info("копия заблокирована и освобождена, count = %u\n", moved.count);
	return 0;
}
/* ANCHOR_END: copy */

static void __exit pin_c_exit(void)
{
}

module_init(pin_c_init);
module_exit(pin_c_exit);
MODULE_DESCRIPTION("Moving an initialized mutex");
MODULE_LICENSE("GPL");
