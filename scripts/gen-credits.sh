#!/usr/bin/env bash

# This scripts generates the CREDITS file in the repository root.
#
# The generated file contains a list of all contributors to the Ferrit project,
# including those who contributed to the original Libreddit project
# (https://github.com/spikecodes/libreddit).
#
# We use git-log to surface the names and emails of all authors and committers,
# and grep will filter any automated commits due to GitHub.

set -o pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../" || exit 1
git --no-pager log --pretty='%an <%ae>%n%cn <%ce>' master \
    | sort -t'<' -u -k1,1 -k2,2 \
    | grep -Fv -- 'GitHub <noreply@github.com>' \
    > CREDITS
