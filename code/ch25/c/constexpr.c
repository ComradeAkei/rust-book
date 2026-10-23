/* Глава 25: constexpr из C23 — только объекты. Размер массива из
 * constexpr годится, а элемент constexpr-массива и вызов функции в
 * константном выражении — нет: функций, вычисляемых при компиляции, в C
 * нет. */
// ANCHOR: constexpr
constexpr int N = 16;
static unsigned char buf[N]; /* теперь можно */

constexpr unsigned GAINS[4] = {100, 200, 400, 800};
_Static_assert(GAINS[2] == 400, "третий коэффициент"); /* не константа */

static unsigned square(unsigned x) { return x * x; }
constexpr unsigned SQ = square(4); /* функцию не вызвать */
// ANCHOR_END: constexpr

int main(void) { return buf[0] + (int)SQ; }
