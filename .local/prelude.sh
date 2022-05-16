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
