/* COR24 I2C support.
 *
 * Bit-banged I2C master over the FPGA's two-line GPIO at I2CBASE.
 * Adjusted from the original i2cspi/tmp101 source for tc24r:
 *   - K&R parameter declarations rewritten as ANSI prototypes
 *   - `register` storage class dropped (tc24r rejects)
 *   - empty-body loops `while (t--);` braced as `while (t--) {}`
 */
#include "i2cio.h"
#include "libi2c.h"

/* I2C (half, quarter)-clock-period delays */
static void hclkdlay(void)
{
    int t;

    t = 22;
    while (t--) {}
}

static void qclkdlay(void)
{
    int t;

    t = 11;
    while (t--) {}
}

/* Set clock high and wait for it to be high */
static int clkhiw(void)
{
    int t;

    *(I2CBASE + I2CSCL) = 1;
    t = 100000;
    while (t--) {
        if (*(I2CBASE + I2CSCL)) {
            return 0;
        }
    }

    return 1;
}

/* Set clock low */
static void clklo(void)
{
    *(I2CBASE + I2CSCL) = 0;
}

/* Set data high */
static void dathi(void)
{
    *(I2CBASE + I2CSDA) = 1;
}

/* Set data low */
static void datlo(void)
{
    *(I2CBASE + I2CSDA) = 0;
}

/* Master acknowledge */
static void mack(void)
{
    datlo();
    hclkdlay();
    clkhiw();
    hclkdlay();
    clklo();
}

/* Get slave ack/nak */
static int sack(void)
{
    char b;

    dathi();
    hclkdlay();
    clkhiw();
    hclkdlay();
    b = *(I2CBASE + I2CSDA);
    clklo();

    return b;
}

/* External I2C support interface */

void i2cstart(void)
{
    dathi();
    hclkdlay();
    clkhiw();
    qclkdlay();
    datlo();
    qclkdlay();
    clklo();
}

void i2cstop(void)
{
    datlo();
    hclkdlay();
    clkhiw();
    qclkdlay();
    dathi();
    qclkdlay();
    clklo();
}

char i2cread(void)
{
    char d;
    int i;

    d = 0;
    i = 8;
    while (i--) {
        dathi();
        hclkdlay();
        clkhiw();
        hclkdlay();
        d = (d << 1) | *(I2CBASE + I2CSDA);
        clklo();
    }

    mack();

    return d;
}

int i2cwrite(char d)
{
    int i;

    i = 8;
    while (i--) {
        /* Original used `(d < 0)` to grab the MSB; that depends on char
         * being signed. tc24r treats char as unsigned, so the trick
         * silently sent all-zero bytes. Use an explicit bit extract. */
        *(I2CBASE + I2CSDA) = (d >> 7) & 1;
        d <<= 1;
        hclkdlay();
        clkhiw();
        hclkdlay();
        clklo();
    }

    return sack();
}
