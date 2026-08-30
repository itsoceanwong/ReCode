import * as React from "react";
import { cn } from "@/lib/utils";

export function Badge({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "inline-flex items-center rounded-full border border-[var(--color-border)] bg-[var(--color-muted)]/80 px-2.5 py-0.5 text-[11px] font-medium text-[var(--color-muted-foreground)] tracking-wide transition-colors",
        className,
      )}
      {...props}
    />
  );
}
