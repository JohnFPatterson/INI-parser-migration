/* inih -- tests

This works simply by dumping a bunch of info to standard output, which is
redirected to an output file (baseline_*.txt) and checked into the Git
repository. This baseline file is the test output, so the idea is to check it
once, and if it changes -- look at the diff and see which tests failed.

See unittest.bat and unittest.sh for how to run this (with tcc and gcc,
respectively).

*/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#endif
int User;
char Prev_section[50];

#include "ini_dumper.h"

void parse(const char* fname) {
    static int u = 100;
    int e;

    *Prev_section = '\0';
    e = ini_parse(fname, dumper, &u);
    printf("%s: e=%d user=%d\n", fname, e, User);
    u++;
}

int main(void)
{
#ifdef _WIN32
    _setmode(_fileno(stdout), _O_BINARY);
#endif
    parse("no_file.ini");
    parse("normal.ini");
    parse("bad_section.ini");
    parse("bad_comment.ini");
    parse("user_error.ini");
    parse("multi_line.ini");
    parse("bad_multi.ini");
    parse("bom.ini");
    parse("duplicate_sections.ini");
    parse("no_value.ini");
    parse("long_section.ini");
    parse("long_line.ini");
    parse("name_only_after_error.ini");
    return 0;
}
