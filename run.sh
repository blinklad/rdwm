#!/bin/bash

# Variables
RDWM="rdwm"
TEST_PROGRAM="xeyes"

function build {
	cargo build
}

function start {
	Xephyr -br -ac -noreset -screen 800x600 :1 &
	sleep 1
	DISPLAY=:1 $TEST_PROGRAM
}

function run {
	build
	start
}

run
