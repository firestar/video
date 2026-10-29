"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { apiFetch, getToken } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
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

export default function Sidebar() {
  const [subs, setSubs] = useState<Channel[]>([]);

  useEffect(() => {
    if (!getToken()) return;
    apiFetch<SubEntry[]>("/api/subscriptions")
      .then((rows) => setSubs(rows.map((r) => r.channel)))
      .catch(() => setSubs([]));
  }, []);

  return (
    <aside
      className="sticky top-14 hidden h-[calc(100vh-3.5rem)] w-52 shrink-0 overflow-y-auto border-r border-white/10 p-3 md:block"
      style={{ backgroundColor: "#0f0f0f" }}
    >
      <nav className="flex flex-col gap-1">
        {links.map((link) => (
          <Link
            key={link.href}
            href={link.href}
            className="rounded-md px-3 py-2 text-sm text-white/70 hover:bg-white/5 hover:text-white"
          >
            {link.label}
          </Link>
        ))}
      </nav>

      {subs.length > 0 && (
        <>
          <div className="my-3 h-px bg-white/10" />
          <div className="px-3 pb-1 text-xs font-medium uppercase tracking-wide text-white/40">
            Subscribed
          </div>
          <nav className="flex flex-col gap-0.5">
            {subs.map((c) => (
              <Link
                key={c.id}
                href={`/channel/${c.id}`}
                className="flex items-center gap-2.5 rounded-md px-3 py-1.5 text-sm text-white/70 hover:bg-white/5 hover:text-white"
              >
                <Avatar size="sm" className="size-6">
                  {c.avatar_url ? (
                    <AvatarImage src={c.avatar_url} alt={c.name} />
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
    </aside>
  );
}
