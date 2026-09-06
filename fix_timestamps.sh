find . -path './.git' -prune -o -type f -newermt 'now' -print0 \
    | xargs -0 -r touch
