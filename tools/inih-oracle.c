/* inih differential oracle driver — public API only.
 * Build once per profile with the matching -DINI_* flags (see Makefile).
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <libgen.h>
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#endif
#include "../ini.h"

static int User;
static char Prev_section[50];
static int g_user_val = 100;

#if INI_CUSTOM_ALLOCATOR
void* ini_malloc(size_t size) {
    printf("ini_malloc(%d)\n", (int)size);
    return malloc(size);
}
void ini_free(void* ptr) {
    printf("ini_free()\n");
    free(ptr);
}
void* ini_realloc(void* ptr, size_t size) {
    printf("ini_realloc(%d)\n", (int)size);
    return realloc(ptr, size);
}
#endif

#if INI_HANDLER_LINENO
static int dumper(void* user, const char* section, const char* name,
                  const char* value, int lineno)
#else
static int dumper(void* user, const char* section, const char* name,
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
    printf("... %s%s%s;  line %d\n", name, value ? "=" : "", value ? value : "", lineno);
#else
    printf("... %s%s%s;\n", name, value ? "=" : "", value ? value : "");
#endif

    if (!value) {
        return 1;
    }
    return strcmp(name, "user") == 0 && strcmp(value, "parse_error") == 0 ? 0 : 1;
}

#if ORACLE_MODE != 0
static const char* string_display_name(const char* stem) {
    if (strcmp(stem, "empty_string") == 0) return "empty string";
    if (strcmp(stem, "long_line") == 0) return "long line";
    if (strcmp(stem, "long_continued") == 0) return "long continued";
    return stem;
}

static char* read_entire_file(const char* path, size_t* out_len) {
    FILE* f = fopen(path, "rb");
    long sz;
    char* buf;
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    sz = ftell(f);
    if (sz < 0) { fclose(f); return NULL; }
    rewind(f);
    buf = (char*)malloc((size_t)sz + 1);
    if (!buf) { fclose(f); return NULL; }
    if (sz > 0 && fread(buf, 1, (size_t)sz, f) != (size_t)sz) {
        free(buf); fclose(f); return NULL;
    }
    buf[sz] = '\0';
    fclose(f);
    if (out_len) *out_len = (size_t)sz;
    return buf;
}
#endif

#ifndef ORACLE_MODE
#define ORACLE_MODE 0
#endif

int main(int argc, char** argv) {
    const char* fixture = NULL;
    int i;
    char path_copy[4096];
    char* base;
    int e;

#ifdef _WIN32
    _setmode(_fileno(stdout), _O_BINARY);
#endif

    for (i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--sections") == 0) {
            i++;
            continue;
        }
        if (strcmp(argv[i], "--profile") == 0) {
            i++;
            continue;
        }
        if (argv[i][0] == '-') {
            fprintf(stderr, "unknown flag: %s\n", argv[i]);
            return 2;
        }
        fixture = argv[i];
    }
    if (!fixture) {
        fprintf(stderr, "usage: oracle [--sections parse] <fixture>\n");
        return 2;
    }

    strncpy(path_copy, fixture, sizeof(path_copy) - 1);
    path_copy[sizeof(path_copy) - 1] = '\0';
    base = basename(path_copy);

    User = 0;
    *Prev_section = '\0';
    g_user_val = 100;

#if ORACLE_MODE == 0
    if (strcmp(base, "no_file.ini") == 0) {
        e = ini_parse("__inih_no_such_file__.ini", dumper, &g_user_val);
        printf("%s: e=%d user=%d\n", base, e, User);
        return 0;
    }
    e = ini_parse(fixture, dumper, &g_user_val);
    printf("%s: e=%d user=%d\n", base, e, User);
    return 0;
#else
    {
        char stem[256];
        char* content;
        size_t content_len;
        size_t blen = strlen(base);
        if (blen >= sizeof(stem)) blen = sizeof(stem) - 1;
        memcpy(stem, base, blen);
        stem[blen] = '\0';
        if (blen > 4 && strcmp(stem + blen - 4, ".ini") == 0)
            stem[blen - 4] = '\0';
        content = read_entire_file(fixture, &content_len);
        if (!content) {
            fprintf(stderr, "failed to read %s\n", fixture);
            return 1;
        }
        e = ini_parse_string(content, dumper, &g_user_val);
        free(content);
#if ORACLE_MODE == 2
        printf("%s: e=%d\n", string_display_name(stem), e);
#else
        printf("%s: e=%d user=%d\n", string_display_name(stem), e, User);
#endif
        return 0;
    }
#endif
}
