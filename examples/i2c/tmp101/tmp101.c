/* Test TMP101 temperature sensor.
 *
 * Original (cc24/ld24 toolchain) used printf("%.2f\n", ...). tc24r ships
 * no stdio, so this rewrite emits "DD.DD\n" directly to the UART. The
 * output format matches the original closely enough that integration
 * tests can parse it.
 *
 * libi2c.c is `#include`d below: tc24r has no separate linker, so a
 * multi-source demo amalgamates into a single translation unit. The
 * library still ships as its own .c/.h so other demos can include it
 * the same way.
 */
#include "libi2c.c"

/* UART MMIO: data at 0xFF0100, status at 0xFF0101 (bit 7 = TX busy) */
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

/* Set TMP101 resolution to 10 bits */
static void setup101(char a)
{
    i2cstart();
    i2cwrite(a << 1);
    i2cwrite(0x01);
    i2cwrite(0x20);
    i2cstart();
    i2cwrite(a << 1);
    i2cwrite(0x00);
    i2cstop();
}

/* Read TMP101 temperature (12-bit signed, in 1/4 °C steps) */
static int temp101(char a)
{
    unsigned char h, l;

    i2cstart();
    i2cwrite((a << 1) | 1);
    h = i2cread();
    l = i2cread();
    i2cstop();

    return (int)(((h << 2) | (l >> 6)) << 14) >> 14;
}

int main(void)
{
    int t;

    setup101(0x4A);

    while (1) {
        printtemp(temp101(0x4A));

        /* Delay until next read */
        t = -1;
        while (t--) {}
    }

    return 0;
}
