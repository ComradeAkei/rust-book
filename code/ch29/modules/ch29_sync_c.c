// SPDX-License-Identifier: GPL-2.0
/*
 * 29.6: те же четыре работы на C — и третий счётчик, без синхронизации.
 */
#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/atomic.h>
#include <linux/completion.h>
#include <linux/module.h>
#include <linux/smp.h>
#include <linux/spinlock.h>
#include <linux/workqueue.h>

#define WORKERS 4
#define ROUNDS 100000

/* ANCHOR: racy */
static u64 racy;
static u64 locked;
static DEFINE_SPINLOCK(lock);
static atomic64_t counter = ATOMIC64_INIT(0);
static atomic_t finished = ATOMIC_INIT(0);
static DECLARE_COMPLETION(done);
static struct work_struct works[WORKERS];

static void work_fn(struct work_struct *work)
{
	int cpu = raw_smp_processor_id();
	int i;

	for (i = 0; i < ROUNDS; i++) {
		WRITE_ONCE(racy, READ_ONCE(racy) + 1);	/* гонка */
		spin_lock(&lock);
		locked++;
		spin_unlock(&lock);
		atomic64_inc(&counter);
	}
	pr_info("работа %td: CPU %d\n", work - works, cpu);
	if (atomic_inc_return(&finished) == WORKERS)
		complete_all(&done);
}
/* ANCHOR_END: racy */

static int __init sync_c_init(void)
{
	int i;

	for (i = 0; i < WORKERS; i++) {
		INIT_WORK(&works[i], work_fn);
		/* Работы — на разных процессорах, по кругу. */
		queue_work_on(i % num_online_cpus(), system_wq, &works[i]);
	}
	wait_for_completion(&done);
	pr_info("без синхронизации: %llu, под spinlock: %llu, atomic64: %lld, ожидалось %d\n",
		racy, locked, atomic64_read(&counter), WORKERS * ROUNDS);
	return 0;
}

static void __exit sync_c_exit(void)
{
}

module_init(sync_c_init);
module_exit(sync_c_exit);
MODULE_DESCRIPTION("Shared counters in work items");
MODULE_LICENSE("GPL");
