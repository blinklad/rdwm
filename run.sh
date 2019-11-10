#!/bin/bash

# Variables
RDWM="rdwm"
TEST_PROGRAM="xeyes"
args=$@

function build {
	cargo build
}

function start {
	case $args in
		-i|--info) # operational info
			echo info
			log="info"
			;;
		-d|--debug) # debug level
			echo debug
			log="debug"
			;;
		-t|--trace) # trace level
			echo trace
			log="trace"
			;;
		*)			# default 
			log="info,debug,trace"
			;;
	esac

	Xephyr -br -ac -noreset -screen 1920x1080 :3 &
	sleep 1
	# DISPLAY=:4 $TEST_PROGRAM
	DISPLAY=:3 RUST_LOG=$log exec "/home/blinklad/dev/rust/rdwm/target/debug/rdwm" &
	sleep 1
	DISPLAY=:3 xterm
}

function run {
	build
	start
}

run
