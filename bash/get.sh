#!/bin/bash

endpoint="127.0.0.1:3000/api"

curl "$endpoint""$1" -H "Content-Type: application/json" | jq
