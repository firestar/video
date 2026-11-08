"use client";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Menu, Moon, Search, Sun, Upload } from "lucide-react";
import { useAuth } from "@/lib/auth";
import { useTheme } from "@/lib/theme";
import { useSidebarState } from "./SidebarState";
import NotificationsBell from "./NotificationsBell";

export default function Navbar() {
  const { user, logout } = useAuth();
  const { theme, toggle } = useTheme();
  const { setOpen } = useSidebarState();
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const q = query.trim();
    if (!q) return;
    router.push(`/search?q=${encodeURIComponent(q)}`);
  };

  return (
    <header className="sticky top-0 z-50 flex h-14 items-center gap-2 border-b border-border bg-background px-3 sm:px-4">
      <button
        onClick={() => setOpen(true)}
        className="flex size-9 items-center justify-center rounded-md text-foreground hover:bg-muted md:hidden"
        aria-label="Open menu"
      >
        <Menu className="size-5" />
      </button>

      <Link href="/" className="shrink-0 text-lg font-bold tracking-tight">
        YT Clone
      </Link>

      <div className="hidden flex-1 sm:block" />

      {searchOpen ? (
        <form className="flex flex-1 items-center" onSubmit={submit}>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search"
            autoFocus
            className="h-10 w-full rounded-l-md border border-border bg-background px-3 text-sm outline-none"
          />
          <button
            type="submit"
            className="h-10 rounded-r-md border border-l-0 border-border px-3 text-sm"
          >
            <Search className="size-4" />
          </button>
        </form>
      ) : (
        <>
          <form className="hidden max-w-xl flex-1 items-center sm:flex" onSubmit={submit}>
            <div className="relative w-full">
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search"
                className="h-10 w-full rounded-l-md border border-border bg-muted/50 pl-3 pr-10 text-sm outline-none"
              />
              <button
                type="submit"
                className="absolute right-0 top-0 flex h-10 w-10 items-center justify-center rounded-r-md bg-muted text-foreground/60 hover:text-foreground"
              >
                <Search className="size-4" />
              </button>
            </div>
          </form>
          <button
            onClick={() => setSearchOpen(true)}
            className="flex size-9 items-center justify-center rounded-md text-foreground hover:bg-muted sm:hidden"
            aria-label="Search"
          >
            <Search className="size-5" />
          </button>
        </>
      )}

      <div className="flex items-center gap-1 sm:gap-2">
        <button
          onClick={toggle}
          className="flex size-9 items-center justify-center rounded-md text-foreground/70 hover:bg-muted hover:text-foreground"
          aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
        >
          {theme === "dark" ? <Sun className="size-5" /> : <Moon className="size-5" />}
        </button>
        <NotificationsBell />

        <Link href="/upload" className="hidden sm:block">
          <Button size="sm">
            <Upload className="size-4" />
            Upload
          </Button>
        </Link>
        <Link href="/upload" className="sm:hidden">
          <Button size="icon" variant="ghost" className="size-9">
            <Upload className="size-5" />
          </Button>
        </Link>

        {user ? (
          <>
            <span className="hidden max-w-24 truncate text-sm text-foreground/80 md:inline">
              {user.name}
            </span>
            <Button variant="ghost" size="sm" onClick={logout}>
              Logout
            </Button>
          </>
        ) : (
          <Link href="/login" className="text-sm font-medium text-foreground/80 hover:text-foreground">
            Login
          </Link>
        )}
      </div>
    </header>
  );
}
