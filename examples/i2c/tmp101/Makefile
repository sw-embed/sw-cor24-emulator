# TMP101 I2C demo — tc24r + cor24-asm build.
#
# Pipeline:  tmp101.c --tc24r--> tmp101.s
#            tmp101.s --cor24-asm--> tmp101.lgo (+ tmp101.bin + tmp101.lst)
#
# Run:   make           # build tmp101.lgo
#        make clean     # remove generated files

TC24R    := tc24r
COR24ASM := cor24-asm

SRC  := tmp101.c
DEPS := tmp101.c libi2c.c libi2c.h i2cio.h
S    := tmp101.s
BIN  := tmp101.bin
LST  := tmp101.lst
LGO  := tmp101.lgo

.PHONY: all clean
all: $(LGO)

$(S): $(DEPS)
	$(TC24R) $(SRC) -o $(S)

$(LGO) $(BIN) $(LST): $(S)
	$(COR24ASM) $(S) -o $(LGO) --bin $(BIN) --listing $(LST)

clean:
	rm -f $(S) $(BIN) $(LST) $(LGO)
