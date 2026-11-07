"use client";
import { createContext, useContext, useState } from "react";

interface SidebarStateValue {
  open: boolean;
  setOpen: (open: boolean) => void;
}

const SidebarStateContext = createContext<SidebarStateValue>({
  open: false,
  setOpen: () => {},
});

export function SidebarState({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <SidebarStateContext.Provider value={{ open, setOpen }}>
      {children}
    </SidebarStateContext.Provider>
  );
}

export function useSidebarState() {
  return useContext(SidebarStateContext);
}
