/* Введение к главе 2: switch без проверки полноты и с провалом в
 * следующую ветку. Оба промаха компилятор пропускает молча, пока его не
 * попросят предупреждать. */
#include <stdio.h>

/* MODE_SLEEP добавили позже, чем написали mode_name. */
enum mode { MODE_IDLE, MODE_RX, MODE_TX, MODE_SLEEP };

static const char *mode_name(enum mode m)
{
    switch (m) {
    case MODE_IDLE: return "idle";
    case MODE_RX:   return "rx";
    case MODE_TX:   return "tx";
    }
    return "?";
}

/* Длина ответа на команду в байтах. */
static int reply_len(int cmd)
{
    int len;
    switch (cmd) {
    case 0x01: /* статус */
        len = 4;
    case 0x02: /* версия; в ветке выше забыт break */
        len = 8;
        break;
    case 0x10: /* журнал */
        len = 64;
        break;
    default:
        len = -1;
    }
    return len;
}

int main(void)
{
    printf("mode_name(MODE_SLEEP) = %s\n", mode_name(MODE_SLEEP));
    printf("reply_len(0x01) = %d\n", reply_len(0x01));
    return 0;
}
