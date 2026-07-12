#!/bin/bash

set -euo pipefail

REPOS=("coral-xyz/anchor" "solana-foundation/anchor" "otter-sec/anchor")
LABEL="missing-release"
ISSUE_TITLE_PREFIX="Anchor releases not covered by compatibility matrix"

covered_versions=$(grep -oP 'anchor:\s*"\K[^"]+' src/compatibility.rs | sort -u)

echo "=== Covered Anchor versions ==="
echo "$covered_versions"

all_missing=()

for repo in "${REPOS[@]}"; do
    echo "=== Fetching releases from $repo ==="
    releases=$(gh api "repos/$repo/releases" --jq '.[].tag_name' 2>/dev/null || echo "")

    for tag in $releases; do
        version="${tag#anchor-v}"
        version="${version#v}"

        if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            continue
        fi

        if ! echo "$covered_versions" | grep -qxF "$version"; then
            echo "MISSING: $version (from $repo tag $tag)"
            all_missing+=("$version ($repo)")
        fi
    done
done

if [ ${#all_missing[@]} -eq 0 ]; then
    echo "All releases are covered."
    exit 0
fi

missing_list=$(printf '  - %s\n' "${all_missing[@]}")

open_issues=$(gh issue list --repo "$GITHUB_REPOSITORY" \
    --label "$LABEL" \
    --state open \
    --json number,title \
    --jq '.[].title' 2>/dev/null || echo "")

if echo "$open_issues" | grep -qF "$ISSUE_TITLE_PREFIX"; then
    echo "An open issue for missing releases already exists. Skipping."
    exit 0
fi

issue_body=$(cat <<EOF
The following Anchor releases are not present in \`src/compatibility.rs\`:

${missing_list}

Please evaluate these releases and add compatibility rules where evidence is available.

Ref: [sync workflow](workflows/sync.md)
EOF
)

gh issue create \
    --repo "$GITHUB_REPOSITORY" \
    --title "${ISSUE_TITLE_PREFIX}" \
    --label "$LABEL" \
    --body "$issue_body"

echo "Issue created for missing releases."
