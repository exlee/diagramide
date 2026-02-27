#!/bin/bash
./generate_bindgen.sh 2>&1 | grep --line-buffered -v "[wW]arning: " | tee output.log; cat output.log | pbcopy; rm output.log
