#! /usr/bin/env bash

mkdir -p ./fuzzbak/proc/sys/kernel
cp /proc/sys/kernel/core_pattern "./fuzzbak/proc/sys/kernel/core_pattern.bak.$( date +%s )"
echo core | sudo tee /proc/sys/kernel/core_pattern > /dev/null

