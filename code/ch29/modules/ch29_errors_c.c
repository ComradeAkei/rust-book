// SPDX-License-Identifier: GPL-2.0
/*
 * 29.3: те же три ресурса на C — цепочка goto err_* в обратном порядке.
 */
#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/debugfs.h>
#include <linux/err.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/module.h>
#include <linux/slab.h>

/* ANCHOR: param */
static unsigned int fail_at;
module_param(fail_at, uint, 0);
MODULE_PARM_DESC(fail_at, "Fail at step 1-4 (0: do not fail)");
/* ANCHOR_END: param */

static const struct file_operations dummy_fops = {
	.owner = THIS_MODULE,
};

static struct miscdevice misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "ch29-errors-c",
	.fops = &dummy_fops,
};

static void *buf;
static struct dentry *dir;

static int inject(unsigned int step, int err)
{
	if (fail_at != step)
		return 0;
	pr_info("шаг %u: отказ %d\n", step, err);
	return err;
}

/* ANCHOR: init */
static int __init errors_init(void)
{
	int ret;

	ret = inject(1, -ENOMEM);
	if (ret)
		return ret;
	buf = kmalloc(4096, GFP_KERNEL);
	if (!buf)
		return -ENOMEM;

	ret = inject(2, -EBUSY);
	if (ret)
		goto err_buf;
	ret = misc_register(&misc);
	if (ret)
		goto err_buf;

	ret = inject(3, -ENOENT);
	if (ret)
		goto err_misc;
	dir = debugfs_create_dir("ch29_errors_c", NULL);

	ret = inject(4, -EINVAL);
	if (ret)
		goto err_dir;
	pr_info("загружен: буфер, /dev/ch29-errors-c, debugfs\n");
	return 0;

err_dir:
	pr_info("освобождаю: каталог debugfs\n");
	debugfs_remove(dir);
err_misc:
	pr_info("освобождаю: /dev/ch29-errors-c\n");
	misc_deregister(&misc);
err_buf:
	pr_info("освобождаю: буфер\n");
	kfree(buf);
	return ret;
}
/* ANCHOR_END: init */

/* ANCHOR: exit */
static void __exit errors_exit(void)
{
	pr_info("освобождаю: каталог debugfs\n");
	debugfs_remove(dir);
	pr_info("освобождаю: /dev/ch29-errors-c\n");
	misc_deregister(&misc);
	pr_info("освобождаю: буфер\n");
	kfree(buf);
}
/* ANCHOR_END: exit */

module_init(errors_init);
module_exit(errors_exit);
MODULE_DESCRIPTION("Error paths with goto");
MODULE_LICENSE("GPL");
