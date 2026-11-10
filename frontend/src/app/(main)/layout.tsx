import Navbar from "@/components/Navbar";
import SidebarSlot from "@/components/SidebarSlot";
import { SidebarState } from "@/components/SidebarState";

// Layout for all normal site pages (the (main) route group): navbar, sidebar
// and main content padding. Pages outside this group (e.g. /embed/[videoId])
// intentionally skip this chrome.
export default function MainLayout({ children }: { children: React.ReactNode }) {
  return (
    <SidebarState>
      <Navbar />
      <div className="flex">
        <SidebarSlot />
        <main className="flex-1 min-w-0 p-4 sm:p-6">{children}</main>
      </div>
    </SidebarState>
  );
}
