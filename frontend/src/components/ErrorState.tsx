"use client";
import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";

interface Props {
  title?: string;
  message?: string;
  onRetry?: () => void;
  children?: React.ReactNode;
}

export default function ErrorState({
  title = "Something went wrong",
  message,
  onRetry,
  children,
}: Props) {
  const [retrying, setRetrying] = useState(false);

  const handleRetry = async () => {
    if (!onRetry) return;
    setRetrying(true);
    try {
      await onRetry();
    } finally {
      setRetrying(false);
    }
  };

  return (
    <div className="flex flex-col items-center gap-4 py-24 text-center">
      <div className="flex size-16 items-center justify-center rounded-full bg-muted">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-muted-foreground">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 8v4" />
          <path d="M12 16h.01" />
        </svg>
      </div>
      <div>
        <h2 className="text-lg font-semibold">{title}</h2>
        {message && <p className="mt-1 text-sm text-muted-foreground">{message}</p>}
      </div>
      {children}
      {onRetry && (
        <Button variant="outline" onClick={handleRetry} disabled={retrying}>
          <RefreshCw className={`size-4 ${retrying ? "animate-spin" : ""}`} />
          {retrying ? "Retrying…" : "Try again"}
        </Button>
      )}
    </div>
  );
}
