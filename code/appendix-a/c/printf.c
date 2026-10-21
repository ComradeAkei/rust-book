/* Приложение A: форматы printf, с которыми сравниваются строки format! в
 * тестах src/strings.rs. Вывод сверяет run.sh. */
#include <stdio.h>
#include <stdlib.h>

int main(void)
{
    printf("[%5d|%-5d|%05d|%+d]\n", 42, 42, 42, 5);
    printf("[%x|%X|%#x|%08x|%#010x|%o]\n", 255, 255, 255, 255, 255, 8);
    printf("[%.3f|%8.2f|%e]\n", 1.23456, 1.23456, 1.23456);
    printf("[%s|%10s|%-10s|%.3s]\n", "abcdef", "abcdef", "abcdef", "abcdef");
    printf("[%c|%%]\n", 'A');
    printf("[%d|%ld|%d]\n", atoi("42abc"), strtol("0xff", NULL, 16), atoi("abc"));
    return 0;
}
