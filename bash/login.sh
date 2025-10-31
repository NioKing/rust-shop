#!/bin/bash

endpoint="http://127.0.0.1:3000/api"
email=""
password=""

for arg in "$@"; do
  case $arg in
  --email=*)
    email="${arg#*=}"
    ;;
  --password=*)
    password="${arg#*=}"
    ;;
  esac
done

if [[ -z "$email" || -z "$password" ]]; then
  echo "Error: --email and --password are required" >&2
  exit 1
fi

echo "$email"
echo "$password"

json_payload=$(jq -n --arg email "$email" --arg password "$password" '{"email": $email, "password": $password}')

curl -X POST "$endpoint/auth/login" \
  -H "Content-Type: application/json" \
  -d "$json_payload" | jq -r '.refresh_token'
