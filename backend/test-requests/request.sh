#!/bin/bash

wget --post-file $1 localhost:2374/command --header "Content-Type: application/json" -qO -
