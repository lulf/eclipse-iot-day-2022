#!/bin/bash
INITIAL_REV=${1:-"0000"}
set -x

for i in $(seq 1 10)
do
    echo "Running simulator for device-sim-${i}"
    drgdfu upload simulated --version ${INITIAL_REV} cloud --http https://http.sandbox.drogue.cloud --application eclipse-iot-day --device device-sim-${i} --password 1234 &
    pids[${i}]=$!
    sleep 1
done

for pid in ${pids[*]}; do
    wait $pid
done

