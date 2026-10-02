# inih C oracle + parity helpers
CC ?= gcc
CFLAGS ?= -Wall -O2
ROOT := $(abspath .)
BUILD := $(ROOT)/build

.PHONY: all oracles asan-oracles hook-trace parity parity-build clean export-check

all: oracles

parity-build: oracles
	cargo build --release --target-dir target

ORACLE_SRC := $(ROOT)/tools/inih-oracle.c $(ROOT)/ini.c
INCLUDES := -I$(ROOT)

define build_oracle
$(BUILD)/oracle-$(1): $(ORACLE_SRC)
	@mkdir -p $(BUILD)
	$(CC) $(CFLAGS) $(INCLUDES) $(2) $(ROOT)/tools/inih-oracle.c $(ROOT)/ini.c -o $$@.tmp
	mv $$@.tmp $$@
endef

$(eval $(call build_oracle,multi,))
$(eval $(call build_oracle,multi_max_line,-DINI_MAX_LINE=20))
$(eval $(call build_oracle,single,-DINI_ALLOW_MULTILINE=0))
$(eval $(call build_oracle,disallow_inline_comments,-DINI_ALLOW_INLINE_COMMENTS=0))
$(eval $(call build_oracle,stop_on_first_error,-DINI_STOP_ON_FIRST_ERROR=1))
$(eval $(call build_oracle,handler_lineno,-DINI_HANDLER_LINENO=1))
$(eval $(call build_oracle,heap,-DINI_USE_STACK=0))
$(eval $(call build_oracle,heap_max_line,-DINI_USE_STACK=0 -DINI_MAX_LINE=20 -DINI_INITIAL_ALLOC=20))
$(eval $(call build_oracle,heap_realloc,-DINI_USE_STACK=0 -DINI_ALLOW_REALLOC=1 -DINI_INITIAL_ALLOC=5))
$(eval $(call build_oracle,heap_realloc_max_line,-DINI_USE_STACK=0 -DINI_MAX_LINE=20 -DINI_ALLOW_REALLOC=1 -DINI_INITIAL_ALLOC=5))
$(eval $(call build_oracle,call_handler_on_new_section,-DINI_CALL_HANDLER_ON_NEW_SECTION=1))
$(eval $(call build_oracle,allow_no_value,-DINI_ALLOW_NO_VALUE=1))
$(eval $(call build_oracle,string,-DORACLE_MODE=1 -DINI_MAX_LINE=20))
$(eval $(call build_oracle,heap_string,-DORACLE_MODE=1 -DINI_USE_STACK=0 -DINI_MAX_LINE=20 -DINI_INITIAL_ALLOC=20))
$(eval $(call build_oracle,alloc,-DORACLE_MODE=2 -DINI_CUSTOM_ALLOCATOR=1 -DINI_USE_STACK=0 -DINI_ALLOW_REALLOC=1 -DINI_INITIAL_ALLOC=12))

ORACLE_BINS := \
  $(BUILD)/oracle-multi \
  $(BUILD)/oracle-multi_max_line \
  $(BUILD)/oracle-single \
  $(BUILD)/oracle-disallow_inline_comments \
  $(BUILD)/oracle-stop_on_first_error \
  $(BUILD)/oracle-handler_lineno \
  $(BUILD)/oracle-heap \
  $(BUILD)/oracle-heap_max_line \
  $(BUILD)/oracle-heap_realloc \
  $(BUILD)/oracle-heap_realloc_max_line \
  $(BUILD)/oracle-call_handler_on_new_section \
  $(BUILD)/oracle-allow_no_value \
  $(BUILD)/oracle-string \
  $(BUILD)/oracle-heap_string \
  $(BUILD)/oracle-alloc

$(BUILD)/oracle: tools/oracle-dispatch.sh $(ORACLE_BINS)
	@mkdir -p $(BUILD)
	cp tools/oracle-dispatch.sh $(BUILD)/oracle
	chmod +x $(BUILD)/oracle

oracles: $(BUILD)/oracle

# AddressSanitizer builds of file-mode oracles (multi as representative + heap variants)
# Prefer gcc for ASan; clang toolchains here may lack clang_rt.asan archives.
ASAN_CC ?= gcc

asan-oracles:
	@mkdir -p $(BUILD)/asan
	$(ASAN_CC) -g -fsanitize=address,undefined -fno-omit-frame-pointer $(INCLUDES) \
	  $(ROOT)/tools/inih-oracle.c $(ROOT)/ini.c -o $(BUILD)/asan/oracle-multi
	$(ASAN_CC) -g -fsanitize=address,undefined -fno-omit-frame-pointer $(INCLUDES) \
	  -DINI_USE_STACK=0 -DINI_ALLOW_REALLOC=1 -DINI_INITIAL_ALLOC=5 \
	  $(ROOT)/tools/inih-oracle.c $(ROOT)/ini.c -o $(BUILD)/asan/oracle-heap_realloc
	$(ASAN_CC) -g -fsanitize=address,undefined -fno-omit-frame-pointer $(INCLUDES) \
	  -DORACLE_MODE=1 -DINI_MAX_LINE=20 \
	  $(ROOT)/tools/inih-oracle.c $(ROOT)/ini.c -o $(BUILD)/asan/oracle-string

$(BUILD)/hook-trace-c: tools/hook-trace.c ini.c ini.h
	@mkdir -p $(BUILD)
	$(CC) $(CFLAGS) $(INCLUDES) -DINI_USE_STACK=0 -DINI_CUSTOM_ALLOCATOR=1 -DINI_ALLOW_REALLOC=1 -DINI_INITIAL_ALLOC=12 \
	  tools/hook-trace.c ini.c -o $@.tmp
	mv $@.tmp $@

hook-trace: $(BUILD)/hook-trace-c

parity: oracles
	cargo build --release --target-dir target
	@mkdir -p $(BUILD)/parity-gate
	printf '%s' '{"status":"completed","loop_count":0,"workspace_roots":["$(ROOT)"]}' \
	  | $(ROOT)/.cursor/hooks/c-rust-parity/parity_gate.py --force

export-check:
	@mkdir -p $(BUILD)
	@grep -E '^INI_API int ini_parse' ini.h | sed -E 's/^INI_API int ([a-zA-Z0-9_]+).*/\1/' | sort -u > $(BUILD)/header-fns.txt
	@# also catch ini_parse_string_length which may wrap
	@grep -oE 'ini_parse[_a-z]*\(' ini.h | tr -d '(' | sort -u > $(BUILD)/header-fns.txt
	nm -g target/release/libinih_ffi.a 2>/dev/null \
	  | awk 'NF>=3 && ($$2=="T" || $$2=="t"){print $$3}' | sed 's/^_//' | sort -u > $(BUILD)/ffi-exports.txt
	@echo "header functions:"; cat $(BUILD)/header-fns.txt
	@echo "missing from FFI:"; comm -23 $(BUILD)/header-fns.txt $(BUILD)/ffi-exports.txt

clean:
	rm -rf $(BUILD)/oracle* $(BUILD)/asan $(BUILD)/hook-trace* $(BUILD)/parity-gate $(BUILD)/header-fns.txt $(BUILD)/ffi-exports.txt
