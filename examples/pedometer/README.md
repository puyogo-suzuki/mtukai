# A pedometer example
This example counts steps by an accelerometer.  
When the button is pushed, the main processor wakes up and shows the measured steps via Serial.

## Requirements
 - ESP32-C6
 - MPU6050 Accelerometer
 - A button and a pull-down resistance

## Connection
### MPU6050 Accelerometer
ESP32-C6 communicates via I2C.
 - SDA - GPIO6
 - SCL - GPIO7
 - MPU6050's AD0 must be connected with GND, and the address will be 0x68.

### Button
Connect with GPIO0.  
The button must be pull-downed (currently, ESP-HAL does not support pull-down input officialy.).

## Reference
AN-2554:  Step Counting Using the ADXL367 by Analog Devices, Inc.  
https://www.analog.com/en/resources/app-notes/an-2554.html  
We disabled dynamic threshold adjustment because it is too conservative in our enviornment.  
In our code, we only evaluate max-min peak difference.