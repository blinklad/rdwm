#!/bin/bash

# Variables
RDWM="rdwm"
DISPLAY=":1"
TEST_PROGRAM="xeyes"

start() {
	Xephyr `-br -ac -noreset -screen 800x600 $DISPLAY`
	sleep 1
	DISPLAY=$DISPLAY xeyes
}
