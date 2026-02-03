import json

def update_tasks():
    with open(".claude/context/task-breakdown.json", "r") as f:
        data = json.load(f)

    new_phase = {
        "id": "phase-5",
        "name": "Frontend Expansion",
        "days": "91-110",
        "status": "pending",
        "progress": "0%",
        "tasks": [
            # Shared Foundation
            {
                "id": "F-001",
                "name": "Setup Monorepo/Structure",
                "description": "Reorganize frontend folder for multiple apps (admin, user-portal, landing)",
                "priority": "high",
                "estimated_hours": 4,
                "dependencies": [],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "F-002",
                "name": "Extract UI Components",
                "description": "Move Shadcn UI components to shared lib",
                "priority": "high",
                "estimated_hours": 4,
                "dependencies": ["F-001"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "F-003",
                "name": "Setup Shared Config",
                "description": "Tailwind, TypeScript, ESLint shared configs",
                "priority": "medium",
                "estimated_hours": 2,
                "dependencies": ["F-001"],
                "assignee": "frontend",
                "status": "pending"
            },
            # Landing Page
            {
                "id": "L-001",
                "name": "Hero Section",
                "description": "Implement responsive hero with animations",
                "priority": "high",
                "estimated_hours": 4,
                "dependencies": ["F-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "L-002",
                "name": "Features Section",
                "description": "Cards grid with hover effects",
                "priority": "medium",
                "estimated_hours": 4,
                "dependencies": ["L-001"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "L-003",
                "name": "How It Works",
                "description": "Interactive step flow",
                "priority": "medium",
                "estimated_hours": 4,
                "dependencies": ["L-001"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "L-004",
                "name": "API Section",
                "description": "Syntax highlighted code block component",
                "priority": "low",
                "estimated_hours": 2,
                "dependencies": ["L-001"],
                "assignee": "frontend",
                "status": "pending"
            },
            # User Portal
            {
                "id": "U-001",
                "name": "Auth Integration",
                "description": "WebAuthn/Passkey implementation",
                "priority": "critical",
                "estimated_hours": 8,
                "dependencies": ["F-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "U-002",
                "name": "Dashboard Layout",
                "description": "Shell with nav and responsive structure",
                "priority": "high",
                "estimated_hours": 4,
                "dependencies": ["F-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "U-003",
                "name": "KYC Flow",
                "description": "Multi-step form for identity verification",
                "priority": "critical",
                "estimated_hours": 8,
                "dependencies": ["U-001", "U-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "U-004",
                "name": "Asset Overview",
                "description": "Balance cards and charts",
                "priority": "high",
                "estimated_hours": 4,
                "dependencies": ["U-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "U-005",
                "name": "Deposit/Withdraw",
                "description": "Forms with validation and API integration",
                "priority": "critical",
                "estimated_hours": 8,
                "dependencies": ["U-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "U-006",
                "name": "Transaction History",
                "description": "Filterable table with status badges",
                "priority": "medium",
                "estimated_hours": 4,
                "dependencies": ["U-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "U-007",
                "name": "Settings Profile",
                "description": "User profile and security settings",
                "priority": "low",
                "estimated_hours": 2,
                "dependencies": ["U-002"],
                "assignee": "frontend",
                "status": "pending"
            },
            # Admin Polish
            {
                "id": "A-001",
                "name": "Dark Mode Fixes",
                "description": "Ensure all components support dark mode",
                "priority": "medium",
                "estimated_hours": 2,
                "dependencies": [],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "A-002",
                "name": "Chart Upgrade",
                "description": "Improve Recharts visualizations",
                "priority": "low",
                "estimated_hours": 2,
                "dependencies": [],
                "assignee": "frontend",
                "status": "pending"
            },
            {
                "id": "A-003",
                "name": "Table Enhancements",
                "description": "Add density toggle, column visibility",
                "priority": "low",
                "estimated_hours": 2,
                "dependencies": [],
                "assignee": "frontend",
                "status": "pending"
            }
        ]
    }

    # Remove existing phase-5 if it exists to avoid duplicates/conflicts during re-runs
    data["phases"] = [p for p in data["phases"] if p["id"] != "phase-5"]
    data["phases"].append(new_phase)

    # Update summary stats
    data["total_tasks"] += len(new_phase["tasks"])
    data["pending_tasks"] += len(new_phase["tasks"])

    with open(".claude/context/task-breakdown.json", "w") as f:
        json.dump(data, f, indent=2)

    print("Successfully added Phase 5 tasks to task-breakdown.json")

if __name__ == "__main__":
    update_tasks()
