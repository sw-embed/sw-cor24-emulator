# Make assembler/code improver, linker, and long line utility

# Tools and rules
CFLAGS=		-O -Wall

# Components
ASOBJ=		as24.o
LDOBJ=		ld24.o
LLOBJ=		longlgo.o

ASSEMBLER=	cor24-as
LINKER=		cor24-ld
LONGLGO=	longlgo

# Targets
all:		$(ASSEMBLER) $(LINKER) $(LONGLGO) sieve.lgo

# sieve.lgo is committed, so it must be reproducible rather than a
# one-off. as24 writes one L record per byte; longlgo packs them into
# the wide records the monitor expects. as24 emits the trailing G record
# only when a symbol named "start" is defined, which sieve.s aliases to
# _main (0x93) — without it the monitor loads but never runs.
sieve.lgo:	sieve.s $(ASSEMBLER) $(LONGLGO)
		./$(ASSEMBLER) < sieve.s | ./$(LONGLGO) > $@

$(ASSEMBLER):	$(ASOBJ)
		$(CC) -o $@ $(ASOBJ)

$(LINKER):	$(LDOBJ)
		$(CC) -o $@ $(LDOBJ)

$(LONGLGO):	$(LLOBJ)
		$(CC) -o $@ $(LLOBJ)

clean:
		rm -f $(ASOBJ) $(LDOBJ) $(LLOBJ)
		rm -f $(ASSEMBLER) $(LINKER) $(LONGLGO)

# Dependencies
