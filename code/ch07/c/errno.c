/* Глава 7, введение: errno перезаписан раньше, чем его прочитали. */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/* Дописать строку в журнал станции. Нет журнала — не беда, молчим. */
static void log_line(const char *msg)
{
    FILE *f = fopen("/var/log/station/events.log", "a");
    if (f) {
        fprintf(f, "%s\n", msg);
        fclose(f);
    }
}

/* ANCHOR: open_config */
static int open_config(const char *path)
{
    int fd = open(path, O_WRONLY);
    if (fd < 0) {
        log_line("не удалось открыть конфигурацию");
        fprintf(stderr, "open %s: %s\n", path, strerror(errno));
    }
    return fd;
}
/* ANCHOR_END: open_config */

int main(void)
{
    /* /tmp — каталог, и open с O_WRONLY вернёт EISDIR. */
    return open_config("/tmp") < 0;
}
