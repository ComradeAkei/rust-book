/* Глава 25: в C `const` — не константа. Размер массива в области файла
 * обязан быть константным выражением, а `const int` им не является. */
// ANCHOR: const_size
const int N = 16;
static unsigned char buf[N]; /* C17: variably modified 'buf' at file scope */
// ANCHOR_END: const_size

int main(void) { return buf[0]; }
