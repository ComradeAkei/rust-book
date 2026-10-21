/* Раздел 1.4: GCC в режиме GNU C сливает a * b + c в одну инструкцию FMA
 * (-ffp-contract=fast), в режиме ISO C (-std=c11) — нет. */
float muladd(float a, float b, float c)
{
    return a * b + c;
}
