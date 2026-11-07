import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { AuthProvider } from "@/lib/auth";
import { ThemeProvider } from "@/lib/theme";
import Navbar from "@/components/Navbar";
import SidebarSlot from "@/components/SidebarSlot";
import { SidebarState } from "@/components/SidebarState";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "YT Clone",
  description: "YouTube clone",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${geistSans.variable} ${geistMono.variable}`} suppressHydrationWarning>
      <body className="min-h-screen bg-background text-foreground">
        <ThemeProvider>
          <AuthProvider>
            <SidebarState>
              <Navbar />
              <div className="flex">
                <SidebarSlot />
                <main className="flex-1 min-w-0 p-4 sm:p-6">{children}</main>
              </div>
            </SidebarState>
          </AuthProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
