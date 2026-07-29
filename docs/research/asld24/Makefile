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
all:		$(ASSEMBLER) $(LINKER) $(LONGLGO)

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
