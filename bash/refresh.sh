#!/bin/bash

endpoint="http://127.0.0.1:3000/api/auth/refresh"

curl -s -X POST "$endpoint" \
  -H "Content-Type: application/json" -H "Authorization: Bearer $1" | jq -r '.refresh_token'
