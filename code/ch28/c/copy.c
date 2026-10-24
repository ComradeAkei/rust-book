/* Раздел 28.2: копия файла на C — та же работа, что ch28::fd::copy.
 *
 * Каждый вызов возвращает -1 и кладёт причину в errno; каждый путь ошибки
 * обязан закрыть то, что уже открыто. Привычный способ — один выход через
 * goto. С -DLEAK второй open возвращается раньше и дескриптор источника
 * остаётся открытым: компилятор молчит, valgrind --track-fds=yes видит.
 *
 * Запуск: copy SRC DST [N] — N попыток подряд (по умолчанию 1). */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

// ANCHOR: copy
static int copy(const char *src, const char *dst)
{
    char buf[4096];
    int rc = -1;
    int out = -1;
    int in = open(src, O_RDONLY | O_CLOEXEC);
    if (in < 0)
        return -1;
    out = open(dst, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0666);
    if (out < 0) {
#ifdef LEAK
        return -1; /* in не закрыт */
#else
        goto done;
#endif
    }
    for (;;) {
        ssize_t n = read(in, buf, sizeof buf);
        if (n < 0 && errno == EINTR)
            continue;
        if (n < 0)
            goto done;
        if (n == 0)
            break;
        for (ssize_t off = 0; off < n;) {
            ssize_t w = write(out, buf + off, (size_t)(n - off));
            if (w < 0 && errno == EINTR)
                continue;
            if (w < 0)
                goto done;
            off += w; /* короткая запись: дописать остаток */
        }
    }
    rc = 0;
done: {
    /* close может затереть errno — сохраняем причину первой ошибки. А
     * ошибка самого close на файле для записи — тоже потеря данных: NFS
     * сообщает о неудавшейся записи только здесь. */
    int saved = errno;
    if (out >= 0 && close(out) < 0 && rc == 0) {
        rc = -1;
        saved = errno;
    }
    close(in);
    errno = saved;
}
    return rc;
}
// ANCHOR_END: copy

int main(int argc, char **argv)
{
    if (argc < 3) {
        fprintf(stderr, "использование: copy SRC DST [N]\n");
        return 2;
    }
    int tries = argc > 3 ? atoi(argv[3]) : 1;
    for (int i = 0; i < tries; i++) {
        if (copy(argv[1], argv[2]) < 0) {
            printf("попытка %d: ошибка: %s\n", i + 1, strerror(errno));
            continue;
        }
        printf("попытка %d: скопировано\n", i + 1);
    }
    return 0;
}
