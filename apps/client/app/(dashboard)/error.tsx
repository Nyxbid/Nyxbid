"use client";

export default function DashboardError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div className="flex min-h-[400px] items-center justify-center">
      <div className="text-center">
        <p className="text-sm font-medium text-foreground">
          Something went wrong
        </p>
        <p className="mt-1 text-xs text-muted">
          {error.message || "An unexpected error occurred."}
        </p>
        <button
          onClick={reset}
          className="mt-4 rounded-md bg-accent px-5 py-2 text-sm font-medium text-white hover:bg-accent/90"
        >
          Try again
        </button>
      </div>
    </div>
  );
}
