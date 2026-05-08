# TMP125 SPI demo — tc24r + cor24-asm build.
#
# Pipeline:
#   tmp125.c        --tc24r--> tmp125_main.s   (body + _start prologue)
#   tmp125_main.s + spixchg.s --cat-->  tmp125.s   (concat: _start MUST be first)
#   tmp125.s        --cor24-asm--> tmp125.lgo (+ tmp125.bin + tmp125.lst)
#
# spixchg.s is hand-written 8x-unrolled SPI exchange (cor24-asm tolerates
# the .text and .globl directives but does not generate them itself).
#
# Run:   make           # build tmp125.lgo
#        make clean     # remove generated files

TC24R    := tc24r
COR24ASM := cor24-asm

C_SRC      := tmp125.c
C_DEPS     := tmp125.c libspi.c libspi.h spiio.h
S_MAIN     := tmp125_main.s
S_SPIXCHG  := spixchg.s
S          := tmp125.s
BIN        := tmp125.bin
LST        := tmp125.lst
LGO        := tmp125.lgo

.PHONY: all clean
all: $(LGO)

$(S_MAIN): $(C_DEPS)
	$(TC24R) $(C_SRC) -o $(S_MAIN)

$(S): $(S_MAIN) $(S_SPIXCHG)
	cat $(S_MAIN) $(S_SPIXCHG) > $(S)

$(LGO) $(BIN) $(LST): $(S)
	$(COR24ASM) $(S) -o $(LGO) --bin $(BIN) --listing $(LST)

clean:
	rm -f $(S_MAIN) $(S) $(BIN) $(LST) $(LGO)
