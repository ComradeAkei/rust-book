/* Глава 7, введение: результат проверки забыт. */
#include <stdio.h>
#include "sensor.h"

int main(void)
{
    int16_t t;
    read_sensor(7, &t);          /* канала 7 нет, ошибка потеряна */
#ifdef VOID_CAST
    (void)read_sensor(7, &t);    /* «я знаю, что делаю» */
#endif
    printf("t = %d\n", t);       /* t не инициализирована */
    return 0;
}
