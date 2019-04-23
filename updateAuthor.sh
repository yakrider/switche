#!/bin/sh

# - details are from https://help.github.com/en/articles/changing-author-info
# - ideally should run on freshely synced/cloned bare repo git clone --bare https://github.com/user/repo.git
# - and after updating the requisite stuff below and running, can push up as git push --force --tags origin 'refs/heads/*'
# - if running in regular repo, it will leave behing a trace/branch for original/refs/heads/develop etc which is hard to cleanup..
#       hence was the suggestion for doing this in a clean cloned temp repo that could be deleted.. oh well not that harmful
# - anyway, if all's good, and REALLY sure, can delete that w git update-ref -d refs/original/refs/heads/develop etc
# - do note that the above wont be doable if there's changes to local repo (unstashed changes etc)
# - and ofc, dont take this lightly as it will overwrite remote history.. and will get flagged up in everyone else's clones they might have

git filter-branch --env-filter '

OLD_EMAIL="dbastakoty@us.ibm.com"
CORRECT_NAME="yakrider"
CORRECT_EMAIL="yakrider@gmail.com"

if [ "$GIT_COMMITTER_EMAIL" = "$OLD_EMAIL" ]
then
    export GIT_COMMITTER_NAME="$CORRECT_NAME"
    export GIT_COMMITTER_EMAIL="$CORRECT_EMAIL"
fi
if [ "$GIT_AUTHOR_EMAIL" = "$OLD_EMAIL" ]
then
    export GIT_AUTHOR_NAME="$CORRECT_NAME"
    export GIT_AUTHOR_EMAIL="$CORRECT_EMAIL"
fi
' --tag-name-filter cat -- --branches --tags