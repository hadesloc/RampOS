# Security Remediation Guide: Purging Secrets from Git History

> **⚠️ WARNING:** This process involves rewriting git history. It is destructive and will affect all collaborators. Ensure everyone has pushed their changes and is ready to re-clone the repository afterwards.

## 1. Preparation

Before starting, ensure you have:
- A backup of your repository
- `bfg` (BFG Repo-Cleaner) installed (recommended) or `git filter-branch`
- Informed all team members to stop working

## 2. Using BFG Repo-Cleaner (Recommended)

BFG is faster and simpler than `git filter-branch`.

```bash
# Download BFG
# java -jar bfg.jar --replace-text passwords.txt my-repo.git

# 1. Create a file containing the secrets to remove
# (Add the old leaked secrets here, one per line)
echo "old_postgres_password" > replacements.txt
echo "old_admin_key" >> replacements.txt
echo "old_encryption_key" >> replacements.txt

# 2. Run BFG to replace secrets with "***REMOVED***"
java -jar bfg.jar --replace-text replacements.txt .

# 3. Clean up the reflog and expire old objects
git reflog expire --expire=now --all && git gc --prune=now --aggressive
```

## 3. Using git filter-branch (Native alternative)

If you cannot use BFG, use the native git command.

```bash
# Remove a specific file from history (e.g., .env if accidentally committed)
git filter-branch --force --index-filter \
  "git rm --cached --ignore-unmatch .env" \
  --prune-empty --tag-name-filter cat -- --all

# Clean up
rm -rf .git/refs/original/
git reflog expire --expire=now --all
git gc --prune=now
```

## 4. Force Push

After rewriting history, you must force push the changes.

```bash
git push origin --force --all
git push origin --force --tags
```

## 5. Instructions for Collaborators

After the history rewrite, all collaborators must:

1.  **NOT** pull the changes to their existing local repository (this will merge dirty history back in).
2.  Clone a fresh copy of the repository:
    ```bash
    git clone <repo-url>
    ```
3.  Or, if they must keep their local folder, reset hard to the new origin:
    ```bash
    git fetch origin
    git reset --hard origin/main
    ```

## 6. Secret Rotation

After purging history, ensure all secrets have been rotated (changed).
Use `scripts/rotate-secrets.sh` to generate new values.
