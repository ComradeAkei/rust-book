#include "sensor.h"

int read_sensor(int channel, int16_t *out)
{
    if (channel > 3)
        return -1;
    *out = (int16_t)(channel * 100);
    return 0;
}
