#!/usr/bin/env bash
set -euf

await() {
	command="${1}"
	exit_code="${2:-0}"
	timeout="${3:-2}"
	set +e
	while true; do
		eval "$command"
		if [ $? -eq "$exit_code" ]; then break; fi
		sleep "$timeout"
	done
	set -e
}

docker_run() {
	name="$1"
	shift
	# shellcheck disable=SC2064
	trap "docker stop $name" INT
	# shellcheck disable=SC2068
	docker run --rm -it --name "$name" ${@}
}

github_clone() {
	path="${1}"
	tag="${2:-main}"
	if [ -d "build/$path" ]; then return; fi
	git clone "git@github.com:${path}" "build/$path"
	cd "build/$path" && git checkout "$tag" && cd -
}

fetch_contract_code() {

	read -r -d '' body <<EOF
{
  "jsonrpc": "2.0",
  "id": 0,
  "method": "eth_getCode",
  "params": [
    "$DATA_EDGE_CONTRACT_ADDRESS",
    "latest"
  ]
}
EOF

	curl --silent --fail "http://127.0.0.1:${HARDHAT_JRPC_PORT}" \
		-H 'Content-Type: application/json; charset=utf-8' \
		-X POST \
		--data-raw "$body"
}

await_contract() {
	timeout="${1:-2}"
	set +e
	while true; do
		response=$(fetch_contract_code)
		exit_code=$?
		if [ $exit_code -eq 0 ]; then
			if jq --exit-status '.result != "0x"' <<<"$response" >/dev/null; then
				break
			else
				echo "DataEdge contract was not deployed yet." >&2
			fi
		else
			echo "Failed to send request to JRPC." >&2
		fi
		sleep "$timeout"
	done
	set -e
}

query_subgraph() {
	curl --silent --fail "http://localhost:${GRAPH_NODE_GRAPHQL_PORT}/subgraphs/name/${DEPLOYMENT_NAME}" \
		-X POST \
		-d '{"query": "{_meta {block {number}}}"}' \
		-H 'Content-Type: application/json; charset=utf-8'
}

await_subgraph() {
	timeout="${1:-2}"
	set +e
	while true; do
		response=$(query_subgraph)
		exit_code=$?
		if [ $exit_code -eq 7 ]; then
			echo "Waiting for graph-node to go live"
		elif jq --exit-status 'has("errors")' <<<"${response}" >/dev/null; then
			if jq --exit-status '.errors[0].message|match("deployment .*? does not exist")' <<<"${response}" >/dev/null; then
				echo "Waiting for subgraph to start"
			else
				echo "Unknown error received from graph-node"
				exit
			fi
		elif jq --exit-status 'has("data")' <<<"$response" >/dev/null; then
			echo "Subgraph was deployed"
			break
		else
			echo "Unknown error"
			exit
		fi
		sleep "$timeout"
	done
	set -e
}
