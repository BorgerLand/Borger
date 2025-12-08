#!/bin/bash
set -e

#for whatever reason, git clean -X (delete gitignored files) and -e (exclude) can't
#be used together, so excluded files must be manually filtered from the dry run
GITIGNORED=$(git clean -dffXn | grep "^Would remove " | sed 's/^Would remove //')
IDE_CONFIG=("rust-analyzer.toml" ".vscode/settings.json")

FILES_TO_CLEAN=()
while IFS= read -r file; do
	[[ -n "$file" ]] || continue
	
	PROTECTED=false
	for pattern in "${IDE_CONFIG[@]}"; do
		if [[ "$file" == "$pattern" ]]; then
			PROTECTED=true
			break
		fi
	done
	
	if [[ "$PROTECTED" == false ]]; then
		FILES_TO_CLEAN+=("$file")
	fi
done <<< "$GITIGNORED"

if [[ ${#FILES_TO_CLEAN[@]} -eq 0 ]]; then
	echo "Workspace is already clean. Nothing to delete."
	exit 0
fi

for file in "${FILES_TO_CLEAN[@]}"; do
	echo "Would remove $file"
done

echo "WARNING: The files listed above (everything in .gitignore excluding IDE config) will be deleted."
echo "Are you sure you want to obliterate? (y/n)"

read -r response
if ! [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
	exit 1
fi

for file in "${FILES_TO_CLEAN[@]}"; do
	rm -rf "$file"
done
