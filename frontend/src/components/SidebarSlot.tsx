"use client";
import { useSidebarState } from "./SidebarState";
import Sidebar from "./Sidebar";

export default function SidebarSlot() {
  const { open, setOpen } = useSidebarState();
  return <Sidebar open={open} onClose={() => setOpen(false)} />;
}
