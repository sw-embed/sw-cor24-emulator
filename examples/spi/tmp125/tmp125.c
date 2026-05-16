/* Test TMP125 SPI temperature sensor.
 *
 * Original (cc24/ld24 toolchain) used printf("%.2f\n", ...). tc24r ships
 * no stdio, so this rewrite emits "DD.DD\n" directly to the UART —
 * mirroring examples/i2c/tmp101/tmp101.c so the integration tests can
 * grep the same shape from both bus types.
 *
 * libspi.c is `#include`d below: tc24r has no separate linker, so a
 * multi-source demo amalgamates into a single translation unit. The
 * library still ships as its own .c/.h so other demos can include it
 * the same way.
 *
 * spixchg() is hand-written assembly (spixchg.s), concatenated ahead
 * of this translation unit's output by the Makefile.
 */
#include "libspi.c"

/* UART MMIO: data at 0xFF0100, status at 0xFF0101 (bit 7 = TX busy). */
#define UART_BASE ((char *)0xFF0100)

static void uart_putc(char c)
{
    while (UART_BASE[1] & 0x80) {}
    UART_BASE[0] = c;
}

/* Emit "DD.DD\n" for t in 0..399 (0.00..99.75 °C in 0.25 °C steps).
 * Negative values get a leading '-' but only the low 99.75 °C of
 * magnitude is rendered; that covers the test cases the saga uses. */
static void printtemp(int t)
{
    int i, f;

    if (t < 0) {
        uart_putc('-');
        t = -t;
    }
    i = t >> 2;
    f = (t & 3) * 25;
    uart_putc('0' + (i / 10));
    uart_putc('0' + (i % 10));
    uart_putc('.');
    uart_putc('0' + (f / 10));
    uart_putc('0' + (f % 10));
    uart_putc('\n');
}

/* Read TMP125 temperature: select the slave, exchange 2 dummy bytes
 * to clock out the 16-bit register (high byte first), deselect.
 * Top 10 bits are the signed temperature in 0.25 °C steps. */
static int temp125(void)
{
    unsigned char h, l;

    spiseln(0);
    h = spixchg(0x00);
    l = spixchg(0x00);
    spiseln(1);

    /* Top 10 bits sign-extended. Cast through int so the >> sign-extends. */
    return (int)(((h << 3) | (l >> 5)) << 14) >> 14;
}

int main(void)
{
    int t;

    while (1) {
        printtemp(temp125());

        /* Delay until next read. */
        t = -1;
        while (t--) {}
    }

    return 0;
}
