"use client";
import { useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { Button } from "@/components/ui/button";

interface MeProfile {
  id: string;
  email: string;
  name: string;
  channel: string;
  bio?: string | null;
  avatarUrl?: string | null;
}

function Banner({ kind, text }: { kind: "ok" | "err"; text: string }) {
  return (
    <div
      className={`mb-4 rounded-md border px-4 py-3 text-sm ${
        kind === "ok"
          ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
          : "border-red-500/40 bg-red-500/10 text-red-600 dark:text-red-400"
      }`}
    >
      {text}
    </div>
  );
}

const inputCls =
  "mt-1 h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30";
const labelCls = "text-xs font-medium text-muted-foreground";

export default function AccountPage() {
  const { user, updateUser } = useAuth();
  const [profile, setProfile] = useState({ name: "", email: "", bio: "" });
  const [loaded, setLoaded] = useState(false);
  const [profileMsg, setProfileMsg] = useState<{ kind: "ok" | "err"; text: string } | null>(null);
  const [savingProfile, setSavingProfile] = useState(false);

  const [pw, setPw] = useState({ current: "", next: "", confirm: "" });
  const [pwMsg, setPwMsg] = useState<{ kind: "ok" | "err"; text: string } | null>(null);
  const [savingPw, setSavingPw] = useState(false);

  useEffect(() => {
    if (!user) return;
    apiFetch<MeProfile>("/api/me")
      .then((me) => {
        setProfile({ name: me.name, email: me.email, bio: me.bio ?? "" });
        setLoaded(true);
      })
      .catch(() => setLoaded(true));
  }, [user]);

  if (!user) {
    return (
      <div className="flex flex-col items-center gap-4 py-24 text-center">
        <h1 className="text-xl font-semibold">Account</h1>
        <p className="text-sm text-muted-foreground">Log in to manage your account.</p>
        <Button onClick={() => { window.location.href = "/login"; }}>Go to Login</Button>
      </div>
    );
  }

  const saveProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    setSavingProfile(true);
    setProfileMsg(null);
    try {
      const res = await apiFetch<{ name: string; email: string }>("/api/me", {
        method: "PATCH",
        body: JSON.stringify({ name: profile.name, email: profile.email, bio: profile.bio }),
      });
      // Refresh the auth context so the navbar reflects the new name/email.
      updateUser({ ...user, name: res.name, email: res.email });
      setProfileMsg({ kind: "ok", text: "Profile saved." });
    } catch (err) {
      setProfileMsg({ kind: "err", text: (err as Error).message });
    } finally {
      setSavingProfile(false);
    }
  };

  const savePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setPwMsg(null);
    if (pw.next.length < 8) {
      setPwMsg({ kind: "err", text: "New password must be at least 8 characters." });
      return;
    }
    if (pw.next !== pw.confirm) {
      setPwMsg({ kind: "err", text: "New passwords do not match." });
      return;
    }
    setSavingPw(true);
    try {
      await apiFetch("/api/auth/change-password", {
        method: "POST",
        body: JSON.stringify({ currentPassword: pw.current, newPassword: pw.next }),
      });
      setPw({ current: "", next: "", confirm: "" });
      setPwMsg({ kind: "ok", text: "Password changed." });
    } catch (err) {
      setPwMsg({ kind: "err", text: (err as Error).message });
    } finally {
      setSavingPw(false);
    }
  };

  return (
    <div className="mx-auto max-w-2xl">
      <div className="mb-6">
        <h1 className="text-2xl font-bold">Account settings</h1>
        <p className="mt-1 text-sm text-muted-foreground">Manage your profile and password.</p>
      </div>

      <section className="mb-8 rounded-lg border border-border bg-background p-5">
        <h2 className="mb-4 text-base font-semibold">Profile</h2>
        {profileMsg && <Banner kind={profileMsg.kind} text={profileMsg.text} />}
        <form onSubmit={saveProfile} className="flex flex-col gap-4">
          <div>
            <label className={labelCls}>Display name</label>
            <input
              className={inputCls}
              value={profile.name}
              onChange={(e) => setProfile({ ...profile, name: e.target.value })}
              required
            />
          </div>
          <div>
            <label className={labelCls}>Email</label>
            <input
              className={inputCls}
              type="email"
              value={profile.email}
              onChange={(e) => setProfile({ ...profile, email: e.target.value })}
              required
            />
          </div>
          <div>
            <label className={labelCls}>Bio</label>
            <textarea
              className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30"
              rows={3}
              placeholder="Tell viewers about yourself"
              value={profile.bio}
              onChange={(e) => setProfile({ ...profile, bio: e.target.value })}
            />
          </div>
          <div>
            <Button type="submit" disabled={savingProfile || !loaded}>
              {savingProfile ? "Saving…" : "Save profile"}
            </Button>
          </div>
        </form>
      </section>

      <section className="rounded-lg border border-border bg-background p-5">
        <h2 className="mb-4 text-base font-semibold">Change password</h2>
        {pwMsg && <Banner kind={pwMsg.kind} text={pwMsg.text} />}
        <form onSubmit={savePassword} className="flex flex-col gap-4">
          <div>
            <label className={labelCls}>Current password</label>
            <input
              className={inputCls}
              type="password"
              value={pw.current}
              onChange={(e) => setPw({ ...pw, current: e.target.value })}
              required
            />
          </div>
          <div>
            <label className={labelCls}>New password</label>
            <input
              className={inputCls}
              type="password"
              value={pw.next}
              onChange={(e) => setPw({ ...pw, next: e.target.value })}
              required
            />
          </div>
          <div>
            <label className={labelCls}>Confirm new password</label>
            <input
              className={inputCls}
              type="password"
              value={pw.confirm}
              onChange={(e) => setPw({ ...pw, confirm: e.target.value })}
              required
            />
          </div>
          <div>
            <Button type="submit" variant="outline" disabled={savingPw}>
              {savingPw ? "Updating…" : "Change password"}
            </Button>
          </div>
        </form>
      </section>
    </div>
  );
}
