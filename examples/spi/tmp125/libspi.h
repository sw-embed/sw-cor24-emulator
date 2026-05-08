/* SPI support interface */
#ifndef _LIBSPI_H_
#define _LIBSPI_H_

/* Select (d=0) or deselect (d=1) the single SPI slave. */
extern void spiseln(char d);

/* Exchange one byte: send `out` MSB-first while receiving the slave's
 * MISO byte over the same 8 SCLK pulses. Implemented in spixchg.s. */
extern char spixchg(char out);

#endif /* _LIBSPI_H_ */
