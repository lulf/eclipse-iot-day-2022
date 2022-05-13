#!/bin/bash

oc delete -f pipeline/simulator.yaml
oc create -f pipeline/simulator.yaml
