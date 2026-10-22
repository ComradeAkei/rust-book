/* Записи конфигурации указывают в буфер строки, который читается заново. */
#include <stdio.h>
#include <string.h>

struct entry {
    const char *key, *val;
};

/* "key=value\n" делится на месте: '=' и '\n' становятся '\0'. */
static int parse(char *line, struct entry *e)
{
    char *eq = strchr(line, '=');
    if (!eq)
        return -1;
    *eq = '\0';
    eq[1 + strcspn(eq + 1, "\n")] = '\0';
    e->key = line;
    e->val = eq + 1;
    return 0;
}

int main(void)
{
    char line[64];
    struct entry cfg[8];
    int n = 0;
    while (n < 8 && fgets(line, sizeof line, stdin))
        if (parse(line, &cfg[n]) == 0)
            n++;
    for (int i = 0; i < n; i++)
        printf("%s = %s\n", cfg[i].key, cfg[i].val);
    return 0;
}
