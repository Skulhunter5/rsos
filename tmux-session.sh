#!/usr/bin/env bash

tmux new-session -d -s "rsos"
tmux rename-window "make"
tmux new-window -n "common" -c "./common/"
tmux new-window -n "kernel" -c "./kernel/"
tmux new-window -n "bootloader" -c "./bootloader/"
tmux select-window -t "make"
tmux attach-session -t "rsos"
