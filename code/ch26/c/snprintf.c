/* Раздел 26.1: snprintf с целым числом из newlib. Ни malloc, ни файлов
 * в программе нет, а в прошивке они есть: run.sh ищет _malloc_r и _sbrk
 * в таблице символов. */
#include <stdio.h>

char buf[32];

int report(int x) {
    return snprintf(buf, sizeof buf, "%d", x);
}

int main(void) {
    return report(3);
}
