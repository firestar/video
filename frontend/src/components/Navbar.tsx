"use client";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/lib/auth";

export default function Navbar() {
  const { user, logout } = useAuth();

  return (
    <header className="sticky top-0 z-50 flex h-14 items-center gap-4 border-b border-white/10 px-4" style={{ backgroundColor: "#0f0f0f" }}>
      <Link href="/" className="text-lg font-bold tracking-tight">
        YT Clone
      </Link>
      <div className="flex-1" />
      <form className="flex flex-1 max-w-xl items-center" onSubmit={(e) => e.preventDefault()}>
        <input
          type="search"
          placeholder="Search"
          className="h-10 w-full rounded-l-md border border-white/20 px-4 text-sm outline-none"
        />
        <button type="submit" className="h-10 rounded-r-md border border-l-0 border-white/20 px-4 text-sm">
          Search
        </button>
      </form>
      <div className="flex items-center gap-2">
        <Link href="/upload">
          <Button>Upload</Button>
        </Link>
        {user ? (
          <>
            <span className="text-sm text-white/80">{user.name}</span>
            <Button variant="ghost" size="sm" onClick={logout}>
              Logout
            </Button>
          </>
        ) : (
          <>
            <Link href="/login" className="text-sm text-white/80 hover:text-white">
              Login
            </Link>
            <Link href="/register" className="text-sm text-white/80 hover:text-white">
              Register
            </Link>
          </>
        )}
      </div>
    </header>
  );
}
