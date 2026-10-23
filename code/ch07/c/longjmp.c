/* Глава 7, введение: longjmp перепрыгивает через освобождение ресурсов. */
#include <pthread.h>
#include <setjmp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static jmp_buf on_error;
static pthread_mutex_t bus = PTHREAD_MUTEX_INITIALIZER;

static void parse(const char *msg)
{
    if (msg[0] != 'A')
        longjmp(on_error, 1);    /* «исключение» */
}

/* ANCHOR: handle */
static int handle(const char *msg)
{
    if (pthread_mutex_trylock(&bus) != 0)
        return -1;               /* шина занята */
    char *copy = malloc(64);
    strcpy(copy, msg);
    parse(copy);                 /* может не вернуться */
    free(copy);
    pthread_mutex_unlock(&bus);
    return 0;
}
/* ANCHOR_END: handle */

int main(void)
{
    const char *msgs[] = {"A1", "B2", "A3"};
    for (volatile int i = 0; i < 3; i++) {
        if (setjmp(on_error)) {
            printf("%s: ошибка разбора\n", msgs[i]);
            continue;
        }
        if (handle(msgs[i]) != 0)
            printf("%s: шина занята\n", msgs[i]);
        else
            printf("%s: принято\n", msgs[i]);
    }
    return 0;
}
