import Link from "next/link";
import { Button } from "@/components/ui/button";

export default function NotFound() {
  return (
    <div className="flex flex-col items-center gap-6 py-32 text-center">
      <div className="text-7xl font-bold text-muted-foreground/30">404</div>
      <div>
        <h1 className="text-2xl font-semibold">Page not found</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          The page you&apos;re looking for doesn&apos;t exist or was moved.
        </p>
      </div>
      <Link href="/">
        <Button>Go home</Button>
      </Link>
    </div>
  );
}
