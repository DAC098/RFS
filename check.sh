#!/bin/bash

package_name="workspace"
package_arg="--workspace"

if [[ -n "$1" ]]; then
	package_name="$1"
	package_arg="-p $1"
fi

echo "checking $package_name"

cargo check $package_arg --tests && 
	cargo check $package_arg --tests --all-features
