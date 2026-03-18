#!/bin/sh

KEY_ID="$ZELA_PROJECT_KEY_ID"
KEY_SECRET="$ZELA_PROJECT_KEY_SECRET"

PROCEDURE="${1}"
PARAMS="${2}"

usage() {
	echo "usage: ZELA_PROJECT_KEY_ID=key_id ZELA_PROJECT_KEY_SECRET=key_secret run-procedure.sh procedure#revision '{ "json": "params" }'"
}

if [ -z "$PROCEDURE" ] || [ -z "$PARAMS" ] || [ -z "$KEY_ID" ] || [ -z "$KEY_SECRET" ]; then
	usage
	exit 1
fi

token=$(curl -sS --user "$KEY_ID:$KEY_SECRET" \
	--data-urlencode 'grant_type=client_credentials' --data-urlencode 'scope=zela-builder:read zela-builder:write' \
	https://auth.zela.io/realms/zela/protocol/openid-connect/token | jq -r .access_token)
# a little bit stupid but we print the output of the request to stderr and capture timing information on stdout
stats=$(curl -s --write-out '%{stdout}%{time_starttransfer} %{time_pretransfer}' -o /dev/stderr \
	--header "Authorization: Bearer $token" --header 'Content-type: application/json' \
	--data "{ \"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"zela.$PROCEDURE\", \"params\": $PARAMS }" https://executor.zela.io)
# then we compute the subtraction of timing
req_time=$(echo "$stats" | awk '{ print $1 - $2 }')
echo "\nRequest time: ${req_time}s"
