// SPDX-License-Identifier: GPL-2.0
/*
 * Задача 29.C: счётчик /dev/ch29-counter-c — эталон на C.
 *
 * write: десятичное число (необязательный '+' впереди, один '\n' в конце)
 * прибавляется к общему счётчику. Не число или длиннее 23 байт — EINVAL,
 * число больше u64 — ERANGE, переполнение суммы — EOVERFLOW.
 * read: чтение с позиции 0 снимает значение «N\n», продолжение чтения
 * отдаёт тот же снимок.
 */
#include <linux/fs.h>
#include <linux/kernel.h>
#include <linux/miscdevice.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/overflow.h>
#include <linux/slab.h>
#include <linux/uaccess.h>

#define MAX_WRITE 23

/* ANCHOR: all */
static DEFINE_MUTEX(counter_lock);
static u64 counter;

struct reader {
	struct mutex lock;
	char text[24];
	size_t len;
};

static int counter_open(struct inode *inode, struct file *file)
{
	struct reader *r = kzalloc(sizeof(*r), GFP_KERNEL);

	if (!r)
		return -ENOMEM;
	mutex_init(&r->lock);
	file->private_data = r;
	return 0;
}

static int counter_release(struct inode *inode, struct file *file)
{
	struct reader *r = file->private_data;

	mutex_destroy(&r->lock);
	kfree(r);
	return 0;
}

static ssize_t counter_read(struct file *file, char __user *buf, size_t count,
			    loff_t *ppos)
{
	struct reader *r = file->private_data;
	ssize_t ret;

	mutex_lock(&r->lock);
	if (*ppos == 0) {
		mutex_lock(&counter_lock);
		r->len = scnprintf(r->text, sizeof(r->text), "%llu\n", counter);
		mutex_unlock(&counter_lock);
	}
	ret = simple_read_from_buffer(buf, count, ppos, r->text, r->len);
	mutex_unlock(&r->lock);
	return ret;
}

static ssize_t counter_write(struct file *file, const char __user *buf,
			     size_t count, loff_t *ppos)
{
	u64 add, sum;
	int ret;

	if (count > MAX_WRITE)
		return -EINVAL;
	ret = kstrtou64_from_user(buf, count, 10, &add);
	if (ret)
		return ret;
	mutex_lock(&counter_lock);
	if (check_add_overflow(counter, add, &sum)) {
		mutex_unlock(&counter_lock);
		return -EOVERFLOW;
	}
	counter = sum;
	mutex_unlock(&counter_lock);
	return count;
}

static const struct file_operations counter_fops = {
	.owner = THIS_MODULE,
	.open = counter_open,
	.release = counter_release,
	.read = counter_read,
	.write = counter_write,
};

static struct miscdevice counter_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "ch29-counter-c",
	.fops = &counter_fops,
};

static int __init counter_init(void)
{
	return misc_register(&counter_misc);
}

static void __exit counter_exit(void)
{
	misc_deregister(&counter_misc);
}
/* ANCHOR_END: all */

module_init(counter_init);
module_exit(counter_exit);
MODULE_DESCRIPTION("Task 29.C reference counter in C");
MODULE_LICENSE("GPL");
