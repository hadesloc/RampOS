"use client";

import * as React from "react";
import { useRouter } from "@/navigation";
import {
  Calendar,
  CreditCard,
  Settings,
  Smile,
  User,
  LayoutDashboard,
  ArrowLeftRight,
  ShieldAlert,
  BookOpen,
  LogOut,
  Moon,
  Sun,
  Laptop,
  Gavel,
  FileText,
  AlertTriangle,
  FileCheck2,
  Banknote,
  Radio,
} from "lucide-react";

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import { useTheme } from "next-themes";

export function CommandPalette() {
  const [open, setOpen] = React.useState(false);
  const router = useRouter();
  const { setTheme } = useTheme();

  React.useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((open) => !open);
      }
    };

    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, []);

  const runCommand = React.useCallback((command: () => unknown) => {
    setOpen(false);
    command();
  }, []);

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder="Type a command or search..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>
        <CommandGroup heading="Suggestions">
          <CommandItem onSelect={() => runCommand(() => router.push('/'))}>
            <LayoutDashboard className="mr-2 h-4 w-4" />
            <span>Dashboard</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/intents'))}>
            <ArrowLeftRight className="mr-2 h-4 w-4" />
            <span>Intents</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/compliance'))}>
            <ShieldAlert className="mr-2 h-4 w-4" />
            <span>Compliance</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/ledger'))}>
            <BookOpen className="mr-2 h-4 w-4" />
            <span>Ledger</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/rfq'))}>
            <Gavel className="mr-2 h-4 w-4" />
            <span>RFQ Auctions</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/fraud'))}>
            <AlertTriangle className="mr-2 h-4 w-4" />
            <span>Fraud Detection</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/reports'))}>
            <FileText className="mr-2 h-4 w-4" />
            <span>Compliance Reports</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/documents'))}>
            <FileCheck2 className="mr-2 h-4 w-4" />
            <span>Documents</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/limits'))}>
            <Banknote className="mr-2 h-4 w-4" />
            <span>Transaction Limits</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/events'))}>
            <Radio className="mr-2 h-4 w-4" />
            <span>Event Catalog</span>
          </CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading="Settings">
          <CommandItem onSelect={() => runCommand(() => router.push('/users'))}>
            <User className="mr-2 h-4 w-4" />
            <span>Users</span>
            <CommandShortcut>⌘U</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => router.push('/settings'))}>
            <Settings className="mr-2 h-4 w-4" />
            <span>Settings</span>
            <CommandShortcut>⌘S</CommandShortcut>
          </CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading="Theme">
          <CommandItem onSelect={() => runCommand(() => setTheme("light"))}>
            <Sun className="mr-2 h-4 w-4" />
            <span>Light</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => setTheme("dark"))}>
            <Moon className="mr-2 h-4 w-4" />
            <span>Dark</span>
          </CommandItem>
          <CommandItem onSelect={() => runCommand(() => setTheme("system"))}>
            <Laptop className="mr-2 h-4 w-4" />
            <span>System</span>
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
