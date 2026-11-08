"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { apiFetch, getToken } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { cn } from "@/lib/utils";
import type { Channel } from "@/lib/types";

interface SubEntry {
  channel: Channel;
  subscriberCount: number;
}

const links = [
  { href: "/", label: "Home" },
  { href: "/trending", label: "Trending" },
  { href: "/subscriptions", label: "Subscriptions" },
  { href: "/history", label: "History" },
];

const studioLink = { href: "/studio", label: "Studio" };

interface Props {
  open: boolean;
  onClose: () => void;
}

export default function Sidebar({ open, onClose }: Props) {
  const [subs, setSubs] = useState<Channel[]>([]);

  useEffect(() => {
    if (!getToken()) return;
    apiFetch<SubEntry[]>("/api/subscriptions")
      .then((rows) => setSubs(rows.map((r) => r.channel)))
      .catch(() => setSubs([]));
  }, []);

  const content = (
    <>
      <nav className="flex flex-col gap-1">
        {links.map((link) => (
          <Link
            key={link.href}
            href={link.href}
            onClick={onClose}
            className="rounded-md px-3 py-2 text-sm text-foreground/70 hover:bg-muted hover:text-foreground"
          >
            {link.label}
          </Link>
        ))}
        {getToken() && (
          <Link
            href={studioLink.href}
            onClick={onClose}
            className="rounded-md px-3 py-2 text-sm text-foreground/70 hover:bg-muted hover:text-foreground"
          >
            {studioLink.label}
          </Link>
        )}
      </nav>

      {subs.length > 0 && (
        <>
          <div className="my-3 h-px bg-border" />
          <div className="px-3 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Subscribed
          </div>
          <nav className="flex flex-col gap-0.5">
            {subs.map((c) => (
              <Link
                key={c.id}
                href={`/channel/${c.id}`}
                onClick={onClose}
                className="flex items-center gap-2.5 rounded-md px-3 py-1.5 text-sm text-foreground/70 hover:bg-muted hover:text-foreground"
              >
                <Avatar size="sm" className="size-6">
                  {c.avatarUrl ? (
                    <AvatarImage src={c.avatarUrl} alt={c.name} />
                  ) : (
                    <AvatarFallback className="text-xs">
                      {c.name.charAt(0).toUpperCase()}
                    </AvatarFallback>
                  )}
                </Avatar>
                <span className="truncate">{c.name}</span>
              </Link>
            ))}
          </nav>
        </>
      )}
    </>
  );

  return (
    <>
      {/* Desktop: static sidebar */}
      <aside className="sticky top-14 hidden h-[calc(100vh-3.5rem)] w-52 shrink-0 overflow-y-auto border-r border-border bg-background p-3 md:block">
        {content}
      </aside>

      {/* Mobile: drawer */}
      <div
        className={cn(
          "fixed inset-0 z-40 bg-black/50 md:hidden",
          open ? "visible" : "invisible"
        )}
        onClick={onClose}
        aria-hidden
      />
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 w-64 overflow-y-auto border-r border-border bg-background p-3 transition-transform duration-200 md:hidden",
          "top-14",
          open ? "translate-x-0" : "-translate-x-full"
        )}
      >
        {content}
      </aside>
    </>
  );
}
