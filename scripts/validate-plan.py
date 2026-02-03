import json
import os
import sys

def validate_plan():
    required_files = [
        ".claude/context/product-spec.md",
        ".claude/context/implementation-plan.md",
        ".claude/context/task-breakdown.json",
        ".claude/context/user-journeys.json"
    ]

    missing_files = []
    for file_path in required_files:
        if not os.path.exists(file_path):
            missing_files.append(file_path)

    if missing_files:
        print(f"FAILED: Missing required files: {missing_files}")
        return False

    try:
        with open(".claude/context/user-journeys.json", "r") as f:
            data = json.load(f)
            # Check for either 'journeys' or 'phases' (renamed for some reason but logic remains same)
            if not data.get("journeys") and not data.get("phases"):
                 print("FAILED: user-journeys.json missing 'journeys' array")
                 return False
    except json.JSONDecodeError:
        print("FAILED: user-journeys.json is not valid JSON")
        return False
        print("FAILED: task-breakdown.json is not valid JSON")
        return False

    print("PASS: Plan validation successful")
    return True

if __name__ == "__main__":
    if validate_plan():
        sys.exit(0)
    else:
        sys.exit(1)
