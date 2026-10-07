#!/bin/sh
cc -c *.c
ar rc libmy.a *.o
ranlib libmy.a
