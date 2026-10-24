// SPDX-License-Identifier: GPL-2.0
/*
 * 29.7: то же устройство на C — /dev/ch29-notes-c.
 */
#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/slab.h>
#include <linux/uaccess.h>

#define CAPACITY 4096
#define NOTES_LEN _IOR('n', 1, u64)
#define NOTES_CLEAR _IO('n', 2)

/* ANCHOR: fops */
struct notes {
	struct mutex lock;
	size_t len;
	char buf[CAPACITY];
};

static int notes_open(struct inode *inode, struct file *file)
{
	struct notes *n = kzalloc(sizeof(*n), GFP_KERNEL);

	if (!n)
		return -ENOMEM;
	mutex_init(&n->lock);
	file->private_data = n;
	return 0;
}

static int notes_release(struct inode *inode, struct file *file)
{
	struct notes *n = file->private_data;

	pr_info("закрыт, в буфере %zu байт\n", n->len);
	mutex_destroy(&n->lock);
	kfree(n);
	return 0;
}

static ssize_t notes_read(struct file *file, char __user *ubuf, size_t count,
			  loff_t *ppos)
{
	struct notes *n = file->private_data;
	ssize_t ret;

	mutex_lock(&n->lock);
	ret = simple_read_from_buffer(ubuf, count, ppos, n->buf, n->len);
	mutex_unlock(&n->lock);
	return ret;
}

static ssize_t notes_write(struct file *file, const char __user *ubuf,
			   size_t count, loff_t *ppos)
{
	struct notes *n = file->private_data;
	ssize_t ret = count;

	mutex_lock(&n->lock);
	if (n->len + count > CAPACITY) {
		ret = -ENOSPC;
		goto out;
	}
	if (copy_from_user(n->buf + n->len, ubuf, count)) {
		ret = -EFAULT;
		goto out;
	}
	n->len += count;
out:
	mutex_unlock(&n->lock);
	return ret;
}

static long notes_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	struct notes *n = file->private_data;
	u64 len;

	switch (cmd) {
	case NOTES_LEN:
		mutex_lock(&n->lock);
		len = n->len;
		mutex_unlock(&n->lock);
		return put_user(len, (u64 __user *)arg);
	case NOTES_CLEAR:
		mutex_lock(&n->lock);
		n->len = 0;
		mutex_unlock(&n->lock);
		return 0;
	default:
		return -ENOTTY;
	}
}

static const struct file_operations notes_fops = {
	.owner = THIS_MODULE,
	.open = notes_open,
	.release = notes_release,
	.read = notes_read,
	.write = notes_write,
	.unlocked_ioctl = notes_ioctl,
};
/* ANCHOR_END: fops */

static struct miscdevice notes_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "ch29-notes-c",
	.fops = &notes_fops,
};

static int __init notes_init(void)
{
	return misc_register(&notes_misc);
}

static void __exit notes_exit(void)
{
	misc_deregister(&notes_misc);
}

module_init(notes_init);
module_exit(notes_exit);
MODULE_DESCRIPTION("Per-open notes buffer as a misc device");
MODULE_LICENSE("GPL");
