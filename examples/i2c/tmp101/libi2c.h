/* I2C support interface */
#ifndef _LIBI2C_H_
#define _LIBI2C_H_

extern void i2cstart(void);
extern void i2cstop(void);
extern char i2cread(void);
extern int i2cwrite(char d);

#endif /* _LIBI2C_H_ */
