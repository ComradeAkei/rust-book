/* Раздел 4.5: счётчик флагов и данные — может ли запись в одно изменить
 * другое? */
#include <stddef.h>
#include <stdint.h>

/* uint8_t может указывать на что угодно, в том числе на байты *cnt. */
void count_flags(uint32_t *cnt, const uint8_t *data, size_t n)
{
    for (size_t i = 0; i < n; i++)
        *cnt += data[i] == 0x7E;
}

/* uint16_t и uint32_t — разные типы: по правилу strict aliasing запись
 * в *cnt не меняет data[i]. */
void count_flags16(uint32_t *cnt, const uint16_t *data, size_t n)
{
    for (size_t i = 0; i < n; i++)
        *cnt += data[i] == 0x7E;
}

/* restrict: программист обещает, что через cnt и data память не
 * пересекается. Компилятор обещание не проверяет. */
void count_flags_restrict(uint32_t *restrict cnt, const uint8_t *data, size_t n)
{
    for (size_t i = 0; i < n; i++)
        *cnt += data[i] == 0x7E;
}
