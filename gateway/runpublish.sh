#!/bin/sh
./target/release/ble-gateway -d E2:9A:A8:1C:CB:0A --report-interval 10sec | while read -r line; do curl -X POST -H "Content-Type: application/json" -d "$line" -u 'microbit@eclipse-iot-day:hey-rodney' https://http.sandbox.drogue.cloud/v1/foo; done
