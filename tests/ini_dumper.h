/* Shared ini_handler dumper for unittest and the differential oracle.
 * Before including, declare:
 *   int User;                 (or static int User)
 *   char Prev_section[50];    (or static)
 * Optionally #define INI_DUMPER_STATIC for file-local linkage.
 */
#ifndef INI_DUMPER_H
#define INI_DUMPER_H

#include <stdio.h>
#include <string.h>

#include "../ini.h"

#ifdef INI_DUMPER_STATIC
#define INI_DUMPER_API static
#else
#define INI_DUMPER_API
#endif

#if INI_HANDLER_LINENO
INI_DUMPER_API int dumper(void* user, const char* section, const char* name,
                          const char* value, int lineno)
#else
INI_DUMPER_API int dumper(void* user, const char* section, const char* name,
                          const char* value)
#endif
{
    User = *((int*)user);
    if (!name || strcmp(section, Prev_section)) {
        printf("... [%s]\n", section);
        strncpy(Prev_section, section, sizeof(Prev_section));
        Prev_section[sizeof(Prev_section) - 1] = '\0';
    }
    if (!name) {
        return 1;
    }

#if INI_HANDLER_LINENO
    printf("... %s%s%s;  line %d\n", name, value ? "=" : "", value ? value : "",
           lineno);
#else
    printf("... %s%s%s;\n", name, value ? "=" : "", value ? value : "");
#endif

    if (!value) {
        /* Happens when INI_ALLOW_NO_VALUE=1 and line has no value (no '=' or ':') */
        return 1;
    }

    return strcmp(name, "user") == 0 && strcmp(value, "parse_error") == 0 ? 0 : 1;
}

#endif /* INI_DUMPER_H */
