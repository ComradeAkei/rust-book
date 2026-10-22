/* Глава 7, введение: функция сообщает об ошибке кодом возврата. */
#include <stdint.h>

#ifdef MUST_USE
#define CHECK __attribute__((warn_unused_result))
#else
#define CHECK
#endif

/* 0 и значение в *out — или −1, если датчик не ответил. */
CHECK int read_sensor(int channel, int16_t *out);
