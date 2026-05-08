/* COR24 SPI support.
 *
 * Adjusted from the original i2cspi/tmp125 source for tc24r:
 *   - K&R parameter declarations rewritten as ANSI prototypes.
 */
#include "spiio.h"
#include "libspi.h"

/* Select/deselect SPI device (active-low: 0 = selected, 1 = deselected). */
void spiseln(char d)
{
    *(SPIBASE + SPISELN) = d;
}
