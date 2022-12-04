#!/usr/bin/env bash

# This scripts generates the CREDITS file in the repository root, which
# contains a list of all contributors ot the Libreddit project.
#
# We use git-log to surface the names and emails of all authors and committers,
# and grep will filter any automated commits due to GitHub.

set -o pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../" || exit 1
git --no-pager log --pretty='%an <%ae>%n%cn <%ce>' master \
    | sort -t'<' -u -k1,1 -k2,2 \
    | grep -Fv -- 'GitHub <noreply@github.com>' \
    > CREDITS
