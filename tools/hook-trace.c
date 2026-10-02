/* Hook-trace: log ini_handler calls and custom allocator traffic (public API). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../ini.h"

void* ini_malloc(size_t size) {
    printf("M %zu\n", size);
    return malloc(size);
}
void ini_free(void* ptr) {
    printf("F\n");
    free(ptr);
}
void* ini_realloc(void* ptr, size_t size) {
    printf("R %zu\n", size);
    return realloc(ptr, size);
}

static int handler(void* user, const char* section, const char* name, const char* value) {
    (void)user;
    printf("H section=%s name=%s value=%s\n",
           section ? section : "(null)",
           name ? name : "(null)",
           value ? value : "(null)");
    return 1;
}

int main(void) {
    const char* s = "[sec]\nfoo = bar\n  continued\nbazz = buzz\n";
    int e = ini_parse_string(s, handler, NULL);
    printf("RESULT %d\n", e);
    return 0;
}
