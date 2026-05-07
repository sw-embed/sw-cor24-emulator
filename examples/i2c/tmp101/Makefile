# TMP101 I2C demo — tc24r build.
#
# Pipeline:  tmp101.c --tc24r--> tmp101.s
#            tmp101.s --cor24-run --assemble--> tmp101.bin (+ .lst)
#            tmp101.bin --xxd--> tmp101.lgo (loader-go format)
#
# Run:   make           # build tmp101.lgo
#        make clean     # remove generated files

TC24R    := tc24r
COR24RUN := cor24-run

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

$(BIN) $(LST): $(S)
	$(COR24RUN) --assemble $(S) $(BIN) $(LST)

$(LGO): $(BIN)
	{ printf 'L000000'; xxd -p $(BIN) | tr -d '\n' | tr 'a-z' 'A-Z'; printf '\n'; } > $(LGO)

clean:
	rm -f $(S) $(BIN) $(LST) $(LGO)
