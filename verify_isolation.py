import re
import os

# Define the repositories and files to check
repo_path = "crates/ramp-core/src/repository"
files_to_check = [
    "intent.rs",
    "ledger.rs",
    "webhook.rs",
    "user.rs",
    "audit.rs"
]

# Define which tables have tenant_id
tables_with_tenant_id = [
    "intents",
    "ledger_entries",
    "account_balances",
    "users",
    "webhook_events",
    "audit_log"
]

def check_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Regex to find sqlx::query calls
    # This is a simple regex and might miss some complex cases, but good enough for a check
    query_pattern = re.compile(r'sqlx::query(?:_as)?(?:<[^>]+>)?\s*\(\s*r?"(.*?)"', re.DOTALL)

    matches = query_pattern.finditer(content)

    violations = []

    for match in matches:
        query = match.group(1)
        # Normalize whitespace
        query = " ".join(query.split())

        # Check if query touches a tenant table
        found_table = None
        for table in tables_with_tenant_id:
            if re.search(r'\b' + table + r'\b', query, re.IGNORECASE):
                found_table = table
                break

        if found_table:
            # Check if tenant_id is in the WHERE clause or INSERT
            if "INSERT" in query.upper():
                if "tenant_id" not in query:
                    violations.append(f"INSERT into {found_table} missing tenant_id: {query[:50]}...")
            elif "UPDATE" in query.upper():
                if "tenant_id" not in query:
                    violations.append(f"UPDATE {found_table} missing tenant_id filter: {query[:50]}...")
            elif "SELECT" in query.upper():
                if "tenant_id" not in query:
                    violations.append(f"SELECT from {found_table} missing tenant_id filter: {query[:50]}...")
            elif "DELETE" in query.upper():
                 if "tenant_id" not in query:
                    violations.append(f"DELETE from {found_table} missing tenant_id filter: {query[:50]}...")

    return violations

def main():
    print("Checking for tenant isolation violations...")
    all_violations = {}
    for filename in files_to_check:
        filepath = os.path.join(repo_path, filename)
        if os.path.exists(filepath):
            violations = check_file(filepath)
            if violations:
                all_violations[filename] = violations
        else:
            print(f"Warning: {filepath} not found")

    if all_violations:
        print("\nViolations found:")
        for filename, v_list in all_violations.items():
            print(f"\n{filename}:")
            for v in v_list:
                print(f"  - {v}")
        exit(1)
    else:
        print("\nNo violations found!")
        exit(0)

if __name__ == "__main__":
    main()
