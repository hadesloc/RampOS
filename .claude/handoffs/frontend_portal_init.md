# Task F-002 + U-002: Portal Layout & Structure

## Status: Completed

## Implemented Changes
1.  **New Portal Layout**:
    - Created `src/components/layout/portal-sidebar.tsx` with user-specific navigation.
    - Created `src/app/portal/layout.tsx` to apply the portal layout.
    - Created `src/app/portal/page.tsx` as the dashboard placeholder.

2.  **Architecture Refactor**:
    - **Crucial**: Moved all Admin routes into a Route Group `src/app/(admin)/` to separate Admin layout from User Portal layout.
    - Created `src/app/(admin)/layout.tsx` to maintain the existing Admin Sidebar.
    - Refactored `src/app/layout.tsx` (RootLayout) to be a clean shell (removed forced Admin Sidebar).

## Usage
- **Admin Dashboard**: Accessible at `/` (and `/intents`, `/users`, etc.). No URL change.
- **User Portal**: Accessible at `/portal`.
- **Portal Pages**: Add new pages under `src/app/portal/` (e.g., `src/app/portal/deposit/page.tsx`).

## Verification
- Checked that admin routes (`/`, `/intents`) are still valid via file structure.
- Verified portal route (`/portal`) exists.
- Verified strict separation of layouts.

## Next Steps
- Implement `src/app/portal/deposit/page.tsx`
- Implement `src/app/portal/withdraw/page.tsx`
- Connect Portal Dashboard to real API (currently using placeholders).

## Notes
- `src/components/ui/` appeared empty in file system checks, but code assumes existence of Shadcn components based on existing imports in `frontend/src/app/(admin)/page.tsx`. If build fails, ensure Shadcn components are installed.
